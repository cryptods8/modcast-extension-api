use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

// Assuming we have a cache implementation, if not, we'd need to implement or use a caching library
use crate::cache::{get_value, set_value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NeynarCast {
    pub hash: String,
}

#[derive(Debug, Deserialize)]
struct NeynarCastResponse {
    cast: Option<NeynarCast>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedData<T> {
    data: T,
    timestamp: u64,
}

pub async fn fetch_cast_from_neynar(cast_url: &str) -> Result<Option<NeynarCast>, reqwest::Error> {
    // let cache = Cache::new(); // Assuming we have a Cache struct
    let cache_key = format!("neynarCast/url/{}", cast_url);

    let cached_data = get_value::<CachedData<NeynarCast>>(&cache_key)
        .await
        .unwrap_or(None);
    if let Some(cd) = cached_data {
        return Ok(Some(cd.data));
    }

    let neynar_api_key = env::var("NEYNAR_API_KEY").expect("NEYNAR_API_KEY must be set");

    let url = format!(
        "https://api.neynar.com/v2/farcaster/cast?identifier={}&type=url",
        cast_url
    );
    let client = Client::new();
    let resp = client
        .get(&url)
        .header("accept", "application/json")
        .header("api_key", neynar_api_key)
        .send()
        .await
        .unwrap();

    if !resp.status().is_success() {
        eprintln!(
            "Failed to fetch cast from Neynar: {} {}",
            resp.status(),
            resp.status().canonical_reason().unwrap_or("")
        );
        return Ok(None);
    }

    let cast_resp: NeynarCastResponse = resp.json().await?;

    if let Some(cast) = cast_resp.cast {
        let cached_data = CachedData {
            data: cast.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
        };
        let _ = set_value(&cache_key, &cached_data).await;
        Ok(Some(cast))
    } else {
        Ok(None)
    }
}
