use std::{fmt::Debug, sync::Arc};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use graphql_client::{GraphQLQuery, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::airstack::fetch_query;
use crate::routes::{
    cast_embeds_handler::{CastEmbedsRequestQuery, CastType},
    config::Config,
    fetch_cast_from_neynar::fetch_cast_from_neynar,
};

pub async fn get_cast_earnings(
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    Query(params): Query<CastEmbedsRequestQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Check API key
    let api_key = headers
        .get("x-me-api-key")
        .and_then(|value| value.to_str().ok());

    if api_key != Some(&config.api_key) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Unauthorized"})),
        ));
    }

    let earnings = fetch_earnings(params, &config).await?;

    Ok(Json(json!({ "data": earnings })))
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AirstackFarcasterCastEarningsDataResponse {
    farcaster_casts: Option<AirstackFarcasterCastEarningsPayload>,
    farcaster_replies: Option<AirstackFarcasterCastEarningsPayload>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AirstackFarcasterCastEarningsPayload {
    cast: Option<Vec<AirstackFarcasterCastEarnings>>,
    reply: Option<Vec<AirstackFarcasterCastEarnings>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AirstackFarcasterCastEarnings {
    casted_by: AirstackFarcasterCastCastedBy,
    channel: Option<AirstackFarcasterCastChannel>,
    moxie_earnings_split: Vec<AirstackFarcasterCastMoxieEarningsSplit>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AirstackFarcasterCastCastedBy {
    user_id: String,
    profile_image: Option<String>,
    fnames: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AirstackFarcasterCastChannel {
    name: String,
    image_url: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AirstackFarcasterCastMoxieEarningsSplit {
    earnings_amount: f64,
    earner_type: String,
}

#[derive(Serialize, Debug)]
pub struct CastEarningsResponse {
    pub earnings: Earnings,
    pub creator: CreatorInfo,
    pub channel: Option<ChannelInfo>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreatorInfo {
    pub fid: i64,
    pub username: Option<String>,
    pub profile_image: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Earnings {
    pub channel_fans: f64,
    pub creator: f64,
    pub network: f64,
    pub creator_fans: f64,
    pub total: f64,
}

impl Earnings {
    pub fn new() -> Self {
        Self {
            channel_fans: 0.0,
            creator: 0.0,
            network: 0.0,
            creator_fans: 0.0,
            total: 0.0,
        }
    }

    fn add(&mut self, split: &AirstackFarcasterCastMoxieEarningsSplit) {
        match split.earner_type.as_str() {
            "CHANNEL_FANS" => {
                self.channel_fans += split.earnings_amount;
            }
            "CREATOR" => {
                self.creator += split.earnings_amount;
            }
            "NETWORK" => {
                self.network += split.earnings_amount;
            }
            "CREATOR_FANS" => {
                self.creator_fans += split.earnings_amount;
            }
            _ => {
                println!("Unknown earner type: {}", split.earner_type);
            }
        }
        self.total += split.earnings_amount;
    }
}

fn extract_cast_earnings_response(
    data: AirstackFarcasterCastEarningsDataResponse,
) -> Option<CastEarningsResponse> {
    let earnings = data
        .farcaster_casts
        .and_then(|fc| fc.cast.and_then(|casts| casts.into_iter().next()))
        .or_else(|| {
            data.farcaster_replies
                .and_then(|fr| fr.reply.and_then(|replies| replies.into_iter().next()))
        })?;

    Some(CastEarningsResponse {
        earnings: earnings
            .moxie_earnings_split
            .iter()
            .fold(Earnings::new(), |mut acc, split| {
                acc.add(split);
                acc
            }),
        creator: CreatorInfo {
            fid: earnings
                .casted_by
                .user_id
                .parse::<i64>()
                .unwrap_or_default(),
            username: earnings.casted_by.fnames.into_iter().next(),
            profile_image: earnings.casted_by.profile_image,
        },
        channel: earnings.channel.map(|c| ChannelInfo {
            name: c.name,
            image_url: c.image_url,
        }),
    })
}

async fn fetch_earnings(
    params: CastEmbedsRequestQuery,
    config: &Config,
) -> Result<Option<CastEarningsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let api_key = config.airstack_api_key.clone();
    let cast_hash = match (
        params.cast_type.clone(),
        params.cast_hash.clone(),
        params.cast_url.clone(),
    ) {
        (Some(CastType::Reply), None, Some(url)) | (None, None, Some(url)) => {
            let cast_result = fetch_cast_from_neynar(&url).await;
            match cast_result {
                Ok(Some(cast)) => Some(cast.hash),
                Ok(None) => {
                    return Err((
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "Cast not found"})),
                    ))
                }
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": e.to_string()})),
                    ))
                }
            }
        }
        _ => params.cast_hash,
    };
    let res = match (params.cast_type, cast_hash, params.cast_url) {
        (Some(CastType::Cast), Some(hash), _) => {
            let request_body =
                CastEarningsByHashQuery::build_query(cast_earnings_by_hash_query::Variables {
                    hash: hash.to_string(),
                });
            fetch_query::<_, Response<AirstackFarcasterCastEarningsDataResponse>>(
                api_key,
                &request_body,
            )
            .await
        }
        (Some(CastType::Reply), Some(hash), _) => {
            let request_body =
                ReplyEarningsByHashQuery::build_query(reply_earnings_by_hash_query::Variables {
                    hash: hash.to_string(),
                });
            fetch_query::<_, Response<AirstackFarcasterCastEarningsDataResponse>>(
                api_key,
                &request_body,
            )
            .await
        }
        (None, Some(hash), _) => {
            let request_body = CastAndReplyEarningsByHashQuery::build_query(
                cast_and_reply_earnings_by_hash_query::Variables { hash: hash.clone() },
            );
            fetch_query::<_, Response<AirstackFarcasterCastEarningsDataResponse>>(
                api_key,
                &request_body,
            )
            .await
        }
        (Some(CastType::Cast), None, Some(url)) => {
            let request_body =
                CastEarningsByUrlQuery::build_query(cast_earnings_by_url_query::Variables {
                    url: url.clone(),
                });
            fetch_query::<_, Response<AirstackFarcasterCastEarningsDataResponse>>(
                api_key,
                &request_body,
            )
            .await
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid parameters"})),
            ))
        }
    };

    let earnings_result = res
        .map(|r| r.data.and_then(|data| extract_cast_earnings_response(data)))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    return Ok(earnings_result);
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_earnings_query.graphql",
    response_derives = "Debug"
)]
pub struct CastEarningsByHashQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_earnings_query.graphql",
    response_derives = "Debug"
)]
pub struct ReplyEarningsByHashQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_earnings_query.graphql",
    response_derives = "Debug"
)]
pub struct CastEarningsByUrlQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_earnings_query.graphql",
    response_derives = "Debug"
)]
pub struct CastAndReplyEarningsByHashQuery;
