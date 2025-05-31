use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore, mpsc};
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

    const MAX_CONCURRENT_REQUESTS: usize = 50;
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    let (tx, mut rx) = mpsc::channel(100);
    let client = Arc::new(Client::new());
    let visited_urls = Arc::new(Mutex::new(HashSet::new()));

    spawn_scraping_task(
        0,
        start_url,
        client.clone(),
        tx.clone(),
        semaphore.clone(),
        visited_urls.clone(),
    );

    drop(tx);

    let mut all_chapters = Vec::new();
    while let Some((index, html)) = rx.recv().await {
        all_chapters.push((index, html));
    }

    println!(
        "\nCrawl complete. Scraped {} chapters. Sorting and saving to file...",
        all_chapters.len()
    );

    all_chapters.sort_by_key(|(index, _)| *index);

    let combined_html = all_chapters
        .iter()
        .map(|(_, html)| html.as_str())
        .collect::<Vec<_>>()
        .join("<hr />\n");
    let final_html = format!(
        r#"
        <!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Scraped Documentation</title>
        <style>body {{ font-family: sans-serif; line-height: 1.6; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }} h1, h2, h3 {{ line-height: 1.2; }} hr {{ margin: 3rem 0; }}</style>
        </head><body>{}</body></html>
        "#,
        combined_html
    );

    fs::write("scraped_book_concurrent.html", final_html)?;
    println!("Successfully saved content to scraped_book_concurrent.html");

    Ok(())
}

/// Helper function to spawn a new scraping task.
fn spawn_scraping_task(
    index: usize,
    url: Url,
    client: Arc<Client>,
    tx: mpsc::Sender<(usize, String)>,
    semaphore: Arc<Semaphore>,
    visited: Arc<Mutex<HashSet<Url>>>,
) {
    tokio::spawn(async move {
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let mut visited_lock = visited.lock().await;
        if !visited_lock.insert(url.clone()) {
            return;
        }
        drop(visited_lock);

        println!("Scraping chapter {}: {}", index, url);

        match scrape_content(&client, &url).await {
            Ok((html_content, next_url_option)) => {
                if tx.send((index, html_content)).await.is_err() {
                    eprintln!("Failed to send scraped content back to main. Receiver closed.");
                }

                if let Some(next_url) = next_url_option {
                    spawn_scraping_task(index + 1, next_url, client, tx, semaphore, visited);
                }
            }
            Err(e) => {
                eprintln!("Error scraping {}: {}", url, e);
            }
        }
    });
}
