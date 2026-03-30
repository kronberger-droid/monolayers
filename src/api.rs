use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
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

    pub async fn apply_tag(
        &self,
        file_id: &str,
        tag_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint_url = self.base_url.join(&format!(
            "/remote.php/dav/systemtags-relations/files/{file_id}/{tag_id}"
        ))?;
        let response = self
            .client
            .put(endpoint_url)
            .basic_auth(
                self.credentials.username(),
                Some(self.credentials.password()),
            )
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() && status != StatusCode::CONFLICT {
            return Err(format!("unexpected status: {}", status).into());
        }

        Ok(())
    }

    pub async fn delete_tag(
        &self,
        file_id: &str,
        tag_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint_url = self.base_url.join(&format!(
            "/remote.php/dav/systemtags-relations/files/{file_id}/{tag_id}"
        ))?;
        let response = self
            .client
            .delete(endpoint_url)
            .basic_auth(
                self.credentials.username(),
                Some(self.credentials.password()),
            )
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() && status != StatusCode::NOT_FOUND {
            return Err(format!("unexpected status: {}", status).into());
        }

        Ok(())
    }

    pub async fn get_tagged_files(
        &self,
        tag_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let endpoint_url = self.base_url.join(&format!(
            "/remote.php/dav/files/{}/",
            self.credentials.username()
        ))?;

        let xml_body = format!(
            r#"<?xml version="1.0"?>
              <oc:filter-files xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
              <oc:filter-rules>
              <oc:systemtag>{tag_id}</oc:systemtag>
              </oc:filter-rules>
            </oc:filter-files>"#
        );

        let response = self
            .client
            .request(Method::from_bytes(b"REPORT").unwrap(), endpoint_url)
            .basic_auth(
                self.credentials.username(),
                Some(self.credentials.password()),
            )
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let body = response.text().await?;
        let multi: MultiStatus = quick_xml::de::from_str(&body)?;

        let prefix =
            format!("/remote.php/dav/files/{}/", self.credentials.username());

        let paths = multi
            .responses
            .iter()
            .filter_map(|r| r.href.strip_prefix(&prefix))
            .map(String::from)
            .collect();

        Ok(paths)
    }

    pub async fn get_file_id(
        &self,
        path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let endpoint_url = self.base_url.join(&format!(
            "/remote.php/dav/files/{}/{}",
            self.credentials.username(),
            path
        ))?;

        let xml_body = r#"<?xml version="1.0"?>
            <d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
              <d:prop>
                <oc:fileid/>
              </d:prop>
            </d:propfind>"#;

        let response = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), endpoint_url)
            .basic_auth(
                self.credentials.username(),
                Some(self.credentials.password()),
            )
            .header("Depth", "0")
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let body = response.text().await?;

        let file_response: FileIdResponse = quick_xml::de::from_str(&body)?;

        Ok(file_response.response.propstat.prop.fileid)
    }

    pub async fn ensure_tag(
        &self,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(id) = self.find_tag_by_name(name).await? {
            return Ok(id);
        }
        self.create_tag(name).await
    }

    async fn find_tag_by_name(
        &self,
        name: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let endpoint_url = self.base_url.join("/remote.php/dav/systemtags")?;

        let xml_body = r#"<?xml version="1.0"?>
            <d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
              <d:prop>
                <oc:display-name/>
              </d:prop>
            </d:propfind>"#;

        let response = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), endpoint_url)
            .basic_auth(
                self.credentials.username(),
                Some(self.credentials.password()),
            )
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(xml_body)
            .send()
            .await?;

        let body = response.text().await?;
        let list: TagListResponse = quick_xml::de::from_str(&body)?;

        Ok(list.responses.iter().find_map(|r| {
            if r.propstat.prop.display_name.as_deref() == Some(name) {
                r.href
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .map(String::from)
            } else {
                None
            }
        }))
    }
}

#[derive(Deserialize)]
struct MultiStatus {
    #[serde(rename = "response", default)]
    responses: Vec<DavResponse>,
}

#[derive(Deserialize)]
struct DavResponse {
    href: String,
}

#[derive(Deserialize)]
struct TagListResponse {
    #[serde(rename = "response", default)]
    responses: Vec<TagResponse>,
}

#[derive(Deserialize)]
struct TagResponse {
    href: String,
    propstat: PropStat,
}

#[derive(Deserialize)]
struct PropStat {
    prop: TagProp,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Deserialize)]
struct TagProp {
    #[serde(rename = "display-name")]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct FileProp {
    fileid: String,
}

#[derive(Deserialize)]
struct FileIdResponse {
    response: FileIdEntry,
}

#[derive(Deserialize)]
struct FileIdEntry {
    propstat: FileIdPropStat,
}

#[derive(Deserialize)]
struct FileIdPropStat {
    prop: FileProp,
}
