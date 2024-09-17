use std::env;

use redis::{Client, Commands, RedisError, RedisResult};
use serde::{de::DeserializeOwned, Serialize};

// implement a general purpose redis cache -- just get and set is enough
fn get_redis_client() -> RedisResult<Client> {
    let host = env::var("REDIS_HOST").expect("REDIS_HOST must be set");
    let port = env::var("REDIS_PORT").expect("REDIS_PORT must be set");
    let password = env::var("REDIS_PASSWORD").expect("REDIS_PASSWORD must be set");
    let username = env::var("REDIS_USERNAME").unwrap_or("default".to_string());
    let protocol = env::var("REDIS_PROTOCOL").unwrap_or("redis".to_string());

    let connection_string = format!("{}://{}:{}@{}:{}", protocol, username, password, host, port);
    let client = redis::Client::open(connection_string)?;
    Ok(client)
}

pub async fn get_value<V: DeserializeOwned>(key: &str) -> RedisResult<Option<V>> {
    let client = get_redis_client()?;
    let mut con = client.get_connection()?;
    let value: Option<String> = con.get(key)?;
    match value {
        Some(val) => {
            let v: V = serde_json::from_str(&val).map_err(|_e| {
                RedisError::from((
                    redis::ErrorKind::ResponseError,
                    "Failed to deserialize value",
                ))
            })?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

pub async fn set_value<V: Serialize>(key: &str, value: &V) -> RedisResult<()> {
    let client = get_redis_client()?;
    let mut con = client.get_connection()?;
    // let _ = con.set_write_timeout(Some(Duration::from_secs(10)));
    let v = serde_json::to_string(value).unwrap();
    let _: () = con.set(key, v)?;
    Ok(())
}
