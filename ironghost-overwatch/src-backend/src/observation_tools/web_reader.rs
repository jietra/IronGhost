// src/observers/web_reader.rs

use serde::{Serialize, Deserialize};
use reqwest::Client;
use scraper::{Html, Selector};
use readability::extractor;
use std::io::Cursor;
use url::Url;


//
const MAX_PAGE_SIZE: usize = 2 * 1024 * 1024; // 2 MB
//



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPage {
    pub url     : String,
    pub title   : Option<String>,
    pub content : String,
}

pub struct WebReader;

impl WebReader {

    pub fn new() -> Self {
        Self
    }

    pub async fn execute( &self, url: &str ) -> anyhow::Result<WebPage> {
        
        self.execute_jina(url).await
    }

    pub async fn execute_jina( &self, url: &str ) -> anyhow::Result<WebPage> {

        let jina_url: String =
            format!(
                "https://r.jina.ai/http://{}",
                url
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
            );

        let content: String =
            Client::new()
                .get(&jina_url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;

        Ok(
            WebPage {
                url     : url.to_string(),
                title   : None,
                content,
            }
        )
    }

    pub async fn execute_readability( &self, url: &str ) -> anyhow::Result<WebPage> {

        let html: String =
            Client::new()
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;

        if html.len() > MAX_PAGE_SIZE {
            anyhow::bail!(
                "page too large ({} bytes)",
                html.len()
            );
        }

        let mut cursor: Cursor<String> =
            Cursor::new(html);

        let article: extractor::Product =
            extractor::extract(
                &mut cursor,
                &Url::parse(url)?,
            )?;

        let content: String =
            article
                .text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

        Ok(
            WebPage {
                url     : url.to_string(),
                title   : Some(article.title),
                content : content,
            }
        )
    }

    pub async fn execute_raw( &self, url: &str ) -> anyhow::Result<WebPage> {

        let html: String =
            Client::new()
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;

        let document: Html =
            Html::parse_document(&html);

        let title: Option<String> =
            Selector::parse("title")
                .ok()
                .and_then(|selector| {
                    document
                        .select(&selector)
                        .next()
                        .map(|n| n.text().collect::<String>())
                });

        let body_selector: Selector =
            Selector::parse("body")
                .map_err(|e| anyhow::anyhow!("selector parse error: {e}"))?;

        let content: String =
            document
                .select(&body_selector)
                .flat_map(|n| n.text())
                .collect::<Vec<_>>()
                .join(" ");

        let content: String =
            content
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

        Ok(
            WebPage {
                url     : url.to_string(),
                title,
                content,
            }
        )
    }
}