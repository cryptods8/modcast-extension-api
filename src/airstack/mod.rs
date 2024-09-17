use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::time::Duration;

pub async fn fetch_query<IT: ?Sized + Serialize, OT: DeserializeOwned + Debug>(
    api_key: String,
    request_body: &IT,
) -> Result<OT, reqwest::Error> {
    let client = reqwest::Client::builder()
        .user_agent("graphql-rust/0.10.0")
        .default_headers(
            std::iter::once((
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
            ))
            .collect(),
        )
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // log the request_body as json string
    // println!("request_body: {:?}", serde_json::to_string(request_body).unwrap());

    let res = client
        .post("https://api.airstack.xyz/gql")
        .json(&request_body)
        .send()
        .await?;

    // clone res to print it
    // let res_text = res.text().await?;
    // println!("res: {:?}", res_text);
    // let value = serde_json::from_str::<OT>(&res_text).unwrap();
    let value = res.json::<OT>().await?;

    Ok(value)
}
