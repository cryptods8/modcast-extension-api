use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::routes::config::Config;

#[derive(Deserialize)]
pub struct FidRequestQuery {
    handle: Option<String>,
}

#[derive(Serialize)]
pub struct FidResponse {
    fid: u64,
}

pub async fn get_fid(
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    Query(params): Query<FidRequestQuery>,
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

    // get handle from query params
    if params.handle.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Handle is required"})),
        ));
    }

    let fid = fetch_fid_from_wc(&params.handle.unwrap()).await.unwrap();

    if fid.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ));
    }

    // Parse and validate fid
    Ok(Json(json!({"data": FidResponse { fid: fid.unwrap() }})))
}

async fn fetch_fid_from_wc(username: &str) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.warpcast.com/v2/user-by-username?username={}",
        username
    );
    let resp = reqwest::get(&url).await?;
    let result: serde_json::Value = resp.json().await?;

    Ok(result
        .get("result")
        .and_then(|r| r.get("user"))
        .and_then(|u| u.get("fid"))
        .and_then(|f| f.as_u64()))
}
