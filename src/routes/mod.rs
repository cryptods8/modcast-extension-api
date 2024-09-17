use std::{env, sync::Arc};

use axum::{routing::get, Router};

mod cast_embeds_handler;
mod config;
mod far_scores_handler;
mod fetch_cast_from_neynar;
mod fids_handler;
mod user_earnings_handler;
mod cast_earnings_handler;

pub fn api_routes() -> Router {
    Router::new()
        .route(
            "/users/:fid/earnings",
            get(user_earnings_handler::get_user_earnings),
        )
        .route("/fids", get(fids_handler::get_fid))
        .route("/far-scores", get(far_scores_handler::get_far_scores))
        .route("/casts/embeds", get(cast_embeds_handler::get_cast_embeds))
        .route("/casts/earnings", get(cast_earnings_handler::get_cast_earnings))
        .with_state(Arc::new(config::Config {
            api_key: env::var("API_KEY").expect("API_KEY must be set"),
            airstack_api_key: env::var("AIRSTACK_API_KEY").expect("AIRSTACK_API_KEY must be set"),
        }))
}
