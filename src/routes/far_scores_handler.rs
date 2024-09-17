use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use graphql_client::{GraphQLQuery, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::routes::config::Config;

#[derive(Deserialize)]
pub struct FarScoreQuery {
    handle: Option<String>,
}

pub async fn get_far_scores(
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    Query(params): Query<FarScoreQuery>,
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

    if params.handle.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Handle is required"})),
        ));
    }

    let far_scores = fetch_far_scores(params.handle.unwrap(), &config).await;

    match far_scores {
        Ok(far_stats) => Ok(Json(json!({"data": far_stats}))),
        Err(e) => Err(e),
    }
    // Ok(Json(json!({"data": 1 })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FarStatsResponse {
    far_score: f64,
    far_rank: i64,
}

async fn fetch_far_scores(
    handle: String,
    config: &Config,
) -> Result<Option<FarStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let request_body = FarScoresQuery::build_query(far_scores_query::Variables { handle: handle });
    let client = reqwest::Client::builder()
        .user_agent("graphql-rust/0.10.0")
        .default_headers(
            std::iter::once((
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!(
                    "Bearer {}",
                    config.airstack_api_key
                ))
                .unwrap(),
            ))
            .collect(),
        )
        .build()
        .unwrap();

    let res = client
        .post("https://api.airstack.xyz/gql")
        .json(&request_body)
        .send()
        .await
        .unwrap();

    let response_body: Response<far_scores_query::ResponseData> = res.json().await.unwrap();

    let far_stats = response_body
        .data
        .as_ref()
        .and_then(|d| d.socials.social.first())
        .map(|s| FarStatsResponse {
            far_score: s.social_capital.social_capital_score,
            far_rank: s.social_capital.social_capital_rank,
        })
        .or(None);

    Ok(far_stats)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/far_scores_query.graphql",
    response_derives = "Debug"
)]
pub struct FarScoresQuery;
