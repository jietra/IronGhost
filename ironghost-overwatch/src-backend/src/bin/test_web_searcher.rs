// src/bin/test_web_searcher.rs

use std::sync::Arc;

use backend::{
    config::Config,
    observers::web_searcher::{
        BraveWebSearcher,
        WebSearcher,
        WebSearchArtifact
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    dotenvy::dotenv().ok();

    let config: Arc<Config> =
        Arc::new( Config::load()? );

    let searcher: BraveWebSearcher =
        BraveWebSearcher::new(config);

    let results: WebSearchArtifact =
        searcher.execute("owasp top 10").await?;

    println!("{:#?}", results);

    Ok(())
}