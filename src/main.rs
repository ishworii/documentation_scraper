use reqwest::Client;
use scraper::{Html, Selector};
use std::fs;
use url::Url;

async fn scrape_content(client: &Client, url: &Url) -> Result<(String, Option<Url>), String> {
    println!("Scraping {}", url);

    let response_text = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| format!("Request failed for {}: {}", url, e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response from {}:{}", url, e))?;

    let document = Html::parse_document(&response_text);

    let content = Selector::parse("main").unwrap();
    let next_chaper_select = Selector::parse("a[title='Next chapter']").unwrap();

    let chapter_html = if let Some(content_div) = document.select(&content).next() {
        content_div.inner_html()
    } else {
        return Err(format!(
            "Could not find div content on the current page : {}",
            url,
        ));
    };

    let next_chapter_url = if let Some(link_element) = document.select(&next_chaper_select).next() {
        link_element
            .value()
            .attr("href")
            .and_then(|href| url.join(href).ok())
    } else {
        None
    };

    Ok((chapter_html, next_chapter_url))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_url = Url::parse("https://doc.rust-lang.org/stable/book/title-page.html")?;

    let client = Client::new();
    let mut all_chapters_html = Vec::new();
    let mut current_url = Some(start_url);

    while let Some(url_to_scrape) = current_url {
        match scrape_content(&client, &url_to_scrape).await {
            Ok((html_content, next_url_option)) => {
                all_chapters_html.push(html_content);
                current_url = next_url_option;
            }
            Err(e) => {
                eprintln!("Stopping scraper due to an error.");
                break;
            }
        }
    }

    println!(
        "Scraping completed,scraped {} chapters.",
        all_chapters_html.len()
    );
    let combined_html = all_chapters_html.join("<hr/>");
    fs::write("scraped_rust_documentation.html", combined_html)?;
    Ok(())
}
