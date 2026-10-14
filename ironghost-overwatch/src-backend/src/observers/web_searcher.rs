// src/observers/web_searcher.rs

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use reqwest::{Client, Response};
use crate::config::Config;

//
// =========================
// IronGhost Artifacts
// =========================
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchArtifact {
    pub query  : String,
    pub results: Vec<WebSearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title   : String,
    pub url     : String,
    pub snippet : String,

    pub language: Option<String>,
    pub age     : Option<String>,
}

//
// =========================
// Public Trait
// =========================
//

#[async_trait]
pub trait WebSearcher: Send + Sync + std::fmt::Debug
{
    async fn execute( &self, query: &str ) -> anyhow::Result<WebSearchArtifact>;
}

//
// =========================
// Brave implementation
// =========================
//

#[derive(Debug)]
pub struct BraveWebSearcher {
    config: Arc<Config>,
}

impl BraveWebSearcher {
    pub fn new( config: Arc<Config> ) -> Self {
        Self { config }
    }
}

#[async_trait]
impl WebSearcher for BraveWebSearcher {

    async fn execute( &self, query: &str ) -> anyhow::Result<WebSearchArtifact> {

        let client  : Client   = Client::new();
        let response: Response =
            client
                .get( &self.config.brave_api_url )
                .header(
                    "Accept",
                    "application/json",
                )
                .header(
                    "X-Subscription-Token",
                    &self.config.brave_api_key,
                )
                .query(&[
                    ("q", query),
                ])
                .send()
                .await?
                .error_for_status()?;

        let payload: BraveSearchResponse  = response.json().await?;
        let results: Vec<WebSearchResult> =
            payload
                .web
                .map(|web: BraveWebResults| {

                    web.results
                        .into_iter()
                        .map(
                            WebSearchResult::from
                        )
                        .collect()
                })
                .unwrap_or_default();

        Ok(WebSearchArtifact {
            query   : query.to_string(),
            results,
        })
    }
}

//
// =========================
// Brave DTOs
// =========================
//

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title      : String,
    url        : String,
    description: Option<String>,
    language   : Option<String>,
    page_age   : Option<String>,
}

//
// =========================
// Mapping DTO -> Artifact
// =========================
//

impl From<BraveResult> for WebSearchResult {

    fn from(value: BraveResult) -> Self {

        Self {
            title   : value.title,
            url     : value.url,
            snippet : value.description.unwrap_or_default(),
            language: value.language,
            age     : value.page_age,
        }
    }
}