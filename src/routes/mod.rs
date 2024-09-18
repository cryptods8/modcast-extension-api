use std::{env, sync::Arc};

use axum::{routing::get, Router};

mod cast_earnings_handler;
mod cast_embeds_handler;
mod config;
mod far_scores_handler;
mod fetch_cast_from_neynar;
mod fids_handler;
mod user_earnings_handler;

pub fn api_routes() -> Router {
    async fn options_handler() -> impl axum::response::IntoResponse {
        axum::response::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header(
                "Access-Control-Allow-Headers",
                "Authorization, Content-Type, X-Requested-With",
            )
            .header("Access-Control-Max-Age", "1728000")
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization",
            )
            .status(axum::http::StatusCode::NO_CONTENT)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    Router::new()
        .route(
            "/users/:fid/earnings",
            get(user_earnings_handler::get_user_earnings).options(options_handler),
        )
        .route("/fids", get(fids_handler::get_fid).options(options_handler))
        .route(
            "/far-scores",
            get(far_scores_handler::get_far_scores).options(options_handler),
        )
        .route(
            "/casts/embeds",
            get(cast_embeds_handler::get_cast_embeds).options(options_handler),
        )
        .route(
            "/earnings",
            get(cast_earnings_handler::get_cast_earnings).options(options_handler),
        )
        .with_state(Arc::new(config::Config {
            api_key: env::var("API_KEY").expect("API_KEY must be set"),
            airstack_api_key: env::var("AIRSTACK_API_KEY").expect("AIRSTACK_API_KEY must be set"),
        }))
}
