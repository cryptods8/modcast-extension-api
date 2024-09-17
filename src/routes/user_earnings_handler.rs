use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use graphql_client::{GraphQLQuery, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::routes::config::Config;

// Handler for GET /earnings/:fid
pub async fn get_user_earnings(
    State(config): State<Arc<Config>>,
    Path(fid): Path<String>,
    headers: HeaderMap,
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

    // Parse and validate fid
    let fid: u64 = fid.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid user identifier: {}", fid)})),
        )
    })?;

    // Fetch earnings (you'll need to implement this function)
    let earnings = fetch_earnings(fid, &config).await?;

    Ok(Json(json!({"data": earnings})))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AirstackEarningStat {
    all_earnings_amount: f64,
    cast_earnings_amount: f64,
    frame_dev_earnings_amount: f64,
    other_earnings_amount: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserEarnings {
    today: Option<AirstackEarningStat>,
    weekly: Option<AirstackEarningStat>,
    lifetime: Option<AirstackEarningStat>,
}

fn to_airstack_earning_stat(
    stat: Option<&moxie_earnings_query::FarcasterMoxieEarningStatFragment>,
) -> Option<AirstackEarningStat> {
    stat.map(|s| AirstackEarningStat {
        all_earnings_amount: s.all_earnings_amount,
        cast_earnings_amount: s.cast_earnings_amount,
        frame_dev_earnings_amount: s.frame_dev_earnings_amount,
        other_earnings_amount: s.other_earnings_amount,
    })
}

async fn fetch_earnings(
    fid: u64,
    config: &Config,
) -> Result<Option<UserEarnings>, (StatusCode, Json<serde_json::Value>)> {
    let request_body = MoxieEarningsQuery::build_query(moxie_earnings_query::Variables {
        fid: fid.to_string(),
    });
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

    // let response_body = res.json().await.unwrap();
    let response_body: Response<moxie_earnings_query::ResponseData> = res.json().await.unwrap();
    // println!("response_body: {:?}", response_body);
    let earnings = response_body
        .data
        .map(|d| UserEarnings {
            today: to_airstack_earning_stat(d.today.farcaster_moxie_earning_stat.get(0)),
            weekly: to_airstack_earning_stat(d.weekly.farcaster_moxie_earning_stat.get(0)),
            lifetime: to_airstack_earning_stat(d.lifetime.farcaster_moxie_earning_stat.get(0)),
        })
        .or(None);

    Ok(earnings)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/airstack_schema.graphql",
    query_path = "src/gql/moxie_earnings_query.graphql",
    response_derives = "Debug"
)]
pub struct MoxieEarningsQuery;
