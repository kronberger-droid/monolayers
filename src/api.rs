use reqwest::{Client, StatusCode};
use url::Url;

use crate::config::UserCredentials;

pub struct NextcloudClient {
    base_url: Url,
    credentials: UserCredentials,
    client: Client,
}

impl NextcloudClient {
    pub fn new(base_url: Url, credentials: UserCredentials) -> Self {
        let client = Client::new();

        Self {
            base_url,
            credentials,
            client,
        }
    }
    pub async fn create_tag(
        &self,
        tag_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "name": tag_name,
            "userVisible": true,
            "userAssignable": false,
        });

        let endpoint_url = self.base_url.join("remote.php/dav/systemtags")?;

        let response = self
            .client
            .post(endpoint_url)
            .basic_auth(
                self.credentials.username(),
                Some(self.credentials.password()),
            )
            .json(&body)
            .send()
            .await?;

        if response.status() != StatusCode::CREATED {
            return Err(
                format!("unexpected status: {}", response.status()).into()
            );
        }

        let tag_id = response
            .headers()
            .get("content-location")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.rsplit('/').next())
            .map(String::from)
            .ok_or("missing content-location header")?;

        Ok(tag_id)
    }
}
