# High-Performance Concurrent Web Crawler in Rust

![Rust](https://img.shields.io/badge/rust-1.78.0-orange.svg)
![Crates.io](https://img.shields.io/badge/crates-tokio,_reqwest,_scraper-blue.svg)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)

This project is a high-performance, asynchronous web crawler built from the ground up in Rust. It's designed to recursively scrape content from a starting URL, follow specific links (e.g., "Next Chapter"), and aggregate the content into a single, clean HTML file.

This project serves as a practical exercise in modern Rust, covering key concepts in asynchronous programming, state management, and network communication.

## Features

-   **Concurrent by Default:** Utilizes `tokio` to make dozens of network requests in parallel, dramatically speeding up I/O-bound workloads.
-   **Recursive Crawling:** Intelligently follows a chain of links (e.g., from chapter to chapter) until no more links are found.
-   **Polite Concurrency:** Uses a `Semaphore` to limit the number of active requests, preventing the crawler from overwhelming the target server.
-   **Robust HTML Extraction:** Uses the `scraper` crate with CSS selectors to precisely target and extract desired content from complex HTML documents.
-   **Safe Shared State:** Employs `Arc<Mutex<...>>` to safely share data (like the set of visited URLs) across all concurrent tasks without race conditions.
-   **Clean Output:** Aggregates all scraped content and saves it as a single, well-formatted HTML file for easy reading.

## Getting Started

### Prerequisites

-   Rust toolchain (latest stable version recommended). You can install it from [rustup.rs](https://rustup.rs/).

### Installation & Usage

1.  **Clone the repository:**
    ```bash
    git clone git@github.com:ishworii/documentation_scraper.git
    cd documentation_scraper
    ```

2.  **Modify the Target URL (Optional):**
    Open `src/main.rs` and change the `start_url` variable to the webpage you want to begin scraping.

3.  **Build and Run:**
    It is **highly recommended** to run the crawler in release mode for optimal performance.

    ```bash
    cargo run --release
    ```

    The program will print its progress to the console and, upon completion, will generate a file named `scraped_documentation_concurrent.html` in the project directory.

## How It Works: The Architecture

The crawler is designed around a coordinator/worker model using `tokio`'s asynchronous channels and tasks.

#### Key Components

-   **`tokio`**: The asynchronous runtime that executes all tasks.
-   **`reqwest`**: A modern, ergonomic HTTP client for making network requests.
-   **`scraper`**: For parsing HTML and querying elements with CSS selectors.
-   **`url`**: A robust library for parsing and normalizing URLs, crucial for handling relative links (`/about.html`).
-   **`tokio::sync::mpsc`**: A multi-producer, single-consumer channel that acts as the central work queue. Worker tasks produce results, and the `main` function consumes them.
-   **`tokio::sync::Semaphore`**: Controls concurrency by issuing a limited number of "permits." A worker task must acquire a permit before making a network request, ensuring we don't spam the server.
-   **`Arc<Mutex<...>>`**: Allows the `HashSet` of visited URLs to be safely shared and modified by many tasks at once.

#### Data Flow

1.  **Initialization:** The `main` function starts, sets up the shared state (`Arc<Mutex<...>>`), the `Semaphore`, and the `mpsc` channel.
2.  **Ignition:** It spawns a *single* worker task for the starting URL and immediately drops its own `tx` (sender) half of the channel. From now on, `main` is only a listener.
3.  **A Worker's Life:**
    a. A worker task starts for a given URL.
    b. It acquires a permit from the `Semaphore`, waiting if the concurrent limit is reached.
    c. It locks the shared `visited_urls` set to check if the URL has already been processed. If not, it adds its URL to the set and proceeds.
    d. It calls `scrape_chapter` to download and parse the page's HTML.
    e. It sends the scraped HTML content back to the `main` function via its clone of the `tx`.
    f. If it finds a "Next Chapter" link, it **spawns a new worker task** for that link, passing on its clones of the shared state and `tx`.
    g. The worker task finishes, and its `permit` is automatically released.
4.  **Coordination & Shutdown:** The `main` function's loop (`rx.recv().await`) collects all the HTML content sent back by the workers. The loop automatically terminates when the last worker task finishes and drops the final `tx` clone, closing the channel.
5.  **Finalization:** Once the crawl is complete, `main` sorts the collected chapters and writes them to the final HTML file.

## Performance Analysis: An Important Lesson

An interesting discovery was made when comparing the performance of the simple sequential crawler against this concurrent version on a very fast website (`doc.rust-lang.org`).

| Version                  | User Time | System Time | CPU Usage | Total Time (Wall Clock) |
| ------------------------ | --------- | ----------- | --------- | ----------------------- |
| Sequential (Debug)       | 2.39s     | 0.23s       | 30%       | 8.460s                  |
| **Sequential (Release)** | **0.32s** | **0.14s** | **15%** | **2.891s** |
| Concurrent (Debug)       | 1.45s     | 0.12s       | 41%       | 3.790s                  |
| **Concurrent (Release)** | **0.32s** | **0.14s** | **13%** | **3.495s** |

**Insight:** The concurrent version was slightly *slower* in this specific test. This provides a critical lesson in performance engineering: **concurrency has overhead.**

-   **Why?** The target server was extremely fast with very low network latency. The time saved by making requests in parallel was less than the time spent on the overhead of managing all the concurrent machinery (spawning tasks, locking mutexes, sending messages through channels).

-   **When is Concurrency Faster?** Concurrency shines when there is significant I/O wait time. On a slower server or when scraping thousands of pages, this concurrent crawler would be dramatically faster than its sequential counterpart.

## Future Improvements

-   [ ] **Command-Line Interface:** Use the `clap` crate to accept the starting URL and concurrency limit as command-line arguments.
-   [ ] **Respect `robots.txt`:** Implement a basic parser for the target's `robots.txt` file to be a more ethical crawler.
-   [ ] **More Robust Error Handling:** Add retry logic with exponential backoff for failed network requests.
-   [ ] **Different Output Formats:** Add flags to save the output as Markdown, JSON, or into a database like SQLite.
