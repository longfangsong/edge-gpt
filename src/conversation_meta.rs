use std::sync::Arc;

use crate::{util::new_reqwest_client, CookieInFile};
use reqwest::{
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

fn create_conversation_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authority",
        HeaderValue::from_static("edgeservices.bing.com"),
    );
    headers.insert("accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
    headers.insert(
        "accept-language",
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert("cache-control", HeaderValue::from_static("max-age=0"));
    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            "\"Chromium\";v=\"110\", \"Not A(Brand\";v=\"24\", \"Microsoft Edge\";v=\"110\"",
        ),
    );
    headers.insert("sec-ch-ua-arch", HeaderValue::from_static("\"x86\""));
    headers.insert("sec-ch-ua-bitness", HeaderValue::from_static("\"64\""));
    headers.insert(
        "sec-ch-ua-full-version",
        HeaderValue::from_static("\"110.0.1587.69\""),
    );
    headers.insert("sec-ch-ua-full-version-list", HeaderValue::from_static("\"Chromium\";v=\"110.0.5481.192\", \"Not A(Brand\";v=\"24.0.0.0\", \"Microsoft Edge\";v=\"110.0.1587.69\""));
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert("sec-ch-ua-model", HeaderValue::from_static("\"\""));
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static("\"Windows\""),
    );
    headers.insert(
        "sec-ch-ua-platform-version",
        HeaderValue::from_static("\"15.0.0\""),
    );
    headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("none"));
    headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
    headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
    headers.insert("x-edge-shopping-flag", HeaderValue::from_static("1"));
    headers.insert("x-forwarded-for", HeaderValue::from_static("1.1.1.1"));
    headers
}

/// Information of a created conversation
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMeta {
    /// used for identify a conversation
    pub conversation_signature: String,
    /// used for identify a client
    pub client_id: String,
    /// used for identify a conversation
    pub conversation_id: String,
}

impl ConversationMeta {
    /// Create a conversation with provided cookies, return the [`ConversationMeta`] of the created conversation.
    pub async fn create(cookies: &[CookieInFile]) -> Result<ConversationMeta> {
        let uri = "https://edgeservices.bing.com/edgesvc/turing/conversation/create"
            .parse()
            .unwrap();
        let cookie_jar: Jar = Jar::default();
        for CookieInFile { name, value } in cookies {
            cookie_jar.add_cookie_str(&format!("{name}={value}; Domain=bing.com"), &uri)
        }
        let response = new_reqwest_client()
            .cookie_provider(Arc::new(cookie_jar))
            .build()?
            .get(uri)
            .headers(create_conversation_headers())
            .send()
            .await;
        Ok(response?.json().await?)
    }
}

#[derive(Error, Debug)]
pub enum ConversationMetaCreatingError {
    #[error("Failed to send conversation meta creating request")]
    Network,
    #[error("Failed to parse conversation meta creating result")]
    ParseRespond(#[from] serde_json::Error),
}

impl From<reqwest::Error> for ConversationMetaCreatingError {
    fn from(_value: reqwest::Error) -> Self {
        Self::Network
    }
}

pub type Result<T> = std::result::Result<T, ConversationMetaCreatingError>;
