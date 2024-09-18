use std::{fmt::Debug, sync::Arc};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use graphql_client::{GraphQLQuery, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    airstack::fetch_query,
    routes::{config::Config, fetch_cast_from_neynar::fetch_cast_from_neynar},
};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CastType {
    Cast,
    Reply,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastEmbedsRequestQuery {
    pub cast_hash: Option<String>,
    pub cast_url: Option<String>,
    #[serde(rename = "type")]
    pub cast_type: Option<CastType>,
}

pub async fn get_cast_embeds(
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

    let embeds = fetch_embeds(params, &config).await?;

    Ok(Json(
        json!({ "data": embeds.map(|e| json!({ "embeds": e })).or(None) }),
    ))
}

#[derive(Serialize, Deserialize)]
pub struct Embed {
    pub url: Option<String>,
}

async fn fetch_embeds(
    params: CastEmbedsRequestQuery,
    config: &Config,
) -> Result<Option<Vec<Embed>>, (StatusCode, Json<serde_json::Value>)> {
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
    let embeds_result = match (params.cast_type, cast_hash, params.cast_url) {
        (Some(CastType::Cast), Some(hash), _) => {
            let request_body =
                CastEmbedsByHashQuery::build_query(cast_embeds_by_hash_query::Variables {
                    hash: hash.to_string(),
                });
            let res = fetch_query::<_, Response<cast_embeds_by_hash_query::ResponseData>>(
                api_key,
                &request_body,
            )
            .await;
            res.map(|r| {
                r.data
                    .and_then(|d| d.farcaster_casts.cast.first().map(|c| c.embeds.clone()))
                    .or(None)
            })
        }
        (Some(CastType::Reply), Some(hash), _) => {
            let request_body =
                ReplyEmbedsByHashQuery::build_query(reply_embeds_by_hash_query::Variables {
                    hash: hash.to_string(),
                });
            let res = fetch_query::<_, Response<reply_embeds_by_hash_query::ResponseData>>(
                api_key,
                &request_body,
            )
            .await;
            res.map(|r| {
                r.data
                    .and_then(|d| d.farcaster_replies.reply.first().map(|c| c.embeds.clone()))
                    .or(None)
            })
        }
        (None, Some(hash), _) => {
            let request_body = CastAndReplyEmbedsByHashQuery::build_query(
                cast_and_reply_embeds_by_hash_query::Variables { hash: hash.clone() },
            );
            let res =
                fetch_query::<_, Response<cast_and_reply_embeds_by_hash_query::ResponseData>>(
                    api_key,
                    &request_body,
                )
                .await;
            res.map(|r| {
                r.data
                    .and_then(|d| {
                        d.farcaster_casts
                            .cast
                            .first()
                            .map(|c| c.embeds.clone())
                            .or_else(|| d.farcaster_replies.reply.first().map(|r| r.embeds.clone()))
                    })
                    .or(None)
            })
        }
        (Some(CastType::Cast), None, Some(url)) => {
            let request_body =
                CastEmbedsByUrlQuery::build_query(cast_embeds_by_url_query::Variables {
                    url: url.clone(),
                });
            let res = fetch_query::<_, Response<cast_embeds_by_url_query::ResponseData>>(
                api_key,
                &request_body,
            )
            .await;
            res.map(|r| {
                r.data
                    .and_then(|d| d.farcaster_casts.cast.first().map(|c| c.embeds.clone()))
                    .or(None)
            })
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid parameters"})),
            ))
        }
    };

    let embeds = embeds_result.map(|embeds_optional| {
        embeds_optional.map(|embeds| {
            embeds
                .iter()
                .map(|embed| match embed["url"].as_str() {
                    Some(url) => Embed {
                        url: Some(url.to_string()),
                    },
                    _ => Embed { url: None },
                })
                .collect::<Vec<Embed>>()
        })
    });

    return embeds.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Internal server error"})),
        )
    });
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_embeds_query.graphql",
    response_derives = "Debug"
)]
pub struct CastEmbedsByHashQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_embeds_query.graphql",
    response_derives = "Debug"
)]
pub struct ReplyEmbedsByHashQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_embeds_query.graphql",
    response_derives = "Debug"
)]
pub struct CastEmbedsByUrlQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/cast_embeds_query.graphql",
    response_derives = "Debug"
)]
pub struct CastAndReplyEmbedsByHashQuery;

type Map = Value;
