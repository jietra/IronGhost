// src/bin/test_web_reader.rs

use backend::observers::web_reader::{WebReader, WebPage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let reader: WebReader = WebReader::new();

    let page: WebPage = reader
        .execute("https://owasp.org/www-project-top-ten/")
        .await?;

    println!("TITLE:\n{}\n", page.title.as_deref().unwrap_or("<no title>"));

    println!("CONTENT:\n{}\n", page.content);

    Ok(())
}