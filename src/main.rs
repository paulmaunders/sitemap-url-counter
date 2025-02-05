use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Duration;
use std::env;

fn clean_xml_content(content: &str) -> String {
    content
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .trim()
        .to_string()
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("Usage: {} <sitemap_url> [--debug]", args[0]);
        std::process::exit(1);
    }
    let main_sitemap_url = &args[1];
    let debug = args.len() == 3 && args[2] == "--debug";

    let client = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(15))
        .build()
        .context("Failed to build HTTP client")?;
    println!("🌐 Fetching main sitemap from {}", main_sitemap_url);
    let content = fetch_url(&client, main_sitemap_url, debug)?;
    
    let mut sitemap_urls = extract_sitemaps(&content, debug)?;
    if sitemap_urls.is_empty() {
        // If no sub-sitemaps found, treat the main URL as a regular sitemap
        sitemap_urls.push(main_sitemap_url.to_string());
    }

    let pb = ProgressBar::new(sitemap_urls.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} sitemaps ({eta})")?
        .progress_chars("#>-"));

    let mut url_counts = HashMap::new();
    
    for sitemap_url in sitemap_urls {
        if debug {
            println!("[DEBUG] Processing sitemap: {}", sitemap_url);
        }
        let content = fetch_url(&client, &sitemap_url, debug)?;
        let count = count_urls(&content, debug)?;
        if debug {
            println!(
                "[DEBUG] Sitemap {} has {} URLs (content size: {} bytes)",
                sitemap_url,
                count,
                content.len()
            );
        }
        url_counts.insert(sitemap_url.clone(), count);
        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    
    println!("\n📊 Results:");
    let total_urls: usize = url_counts.values().sum();
    for (url, count) in url_counts {
        println!("  {} - {} URLs", url, count);
    }
    println!("\n📈 Total URLs found: {}", total_urls);

    Ok(())
}

fn fetch_url(_client: &reqwest::blocking::Client, url: &str, debug: bool) -> Result<String> {
    use std::process::Command;
    
    // Build the curl command. We use "-s" for silent mode.
    let output = Command::new("curl")
        .arg("-s")
        .arg("-L")
        .arg("-A")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36")
        .arg(url)
        .output()
        .context("Failed to execute curl")?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("curl command failed with status: {:?}", output.status));
    }
    
    let content = String::from_utf8(output.stdout)
        .context("Curl output was not valid UTF-8")?;
    
    // Optionally, add debug info:
    if debug { println!("[DEBUG] Curl fetched content length: {}", content.len()); }
    if debug {
        println!(
            "[DEBUG] Curl fetched content snippet: {}",
            &content[..std::cmp::min(500, content.len())]
        );
    }
    if debug { println!("[DEBUG] Fetched sitemap content. Proceeding to clean XML and parse document..."); }
    io::stdout().flush().unwrap();
    
    Ok(clean_xml_content(&content))
}

fn extract_sitemaps(content: &str, debug: bool) -> Result<Vec<String>> {
    if debug { println!("[DEBUG] Starting sitemap extraction from main sitemap content..."); }
    io::stdout().flush().unwrap();
    let mut buf = Vec::new();

    let mut count: usize = 0;
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.set_message("Counting URLs: 0");

    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    let mut sitemaps = Vec::new();
    let mut in_sitemap = false;
    let mut in_loc = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(event) => {
                match event {
                    Event::Start(ref e) if e.name().as_ref() == b"sitemap" => {
                        in_sitemap = true;
                    },
                    Event::End(ref e) if e.name().as_ref() == b"sitemap" => {
                        in_sitemap = false;
                    },
                    Event::Start(ref e) if in_sitemap && e.name().as_ref() == b"loc" => {
                        in_loc = true;
                    },
                    Event::Text(e) if in_loc => {
                        sitemaps.push(e.unescape()?.into_owned());
                        count += 1;
                        pb.set_message(format!("Counting URLs: {}", count));
                        in_loc = false;
                    },
                    _ => (),
                }
            },
            Err(e) => return Err(e.into()),
        }
        buf.clear();
    }
    pb.finish_and_clear();
    if debug { println!("[DEBUG] Completed sitemap extraction. Found {} sitemaps.", sitemaps.len()); }
    io::stdout().flush().unwrap();
    Ok(sitemaps)
}

fn count_urls(content: &str, debug: bool) -> Result<usize> {
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    let mut count = 0;
    let mut in_url = false;
    let mut in_loc = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"url" => {
                in_url = true;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"url" => {
                in_url = false;
            }
            Ok(Event::Start(ref e)) if in_url && e.name().as_ref() == b"loc" => {
                in_loc = true;
            }
            Ok(Event::Text(e)) if in_loc => {
                let _url_text = e.unescape()?.into_owned();
                count += 1;
                in_loc = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => (),
        }
        buf.clear();
    }
    if debug { println!("[DEBUG] Finished URL count. Found {} URLs.", count); }
    io::stdout().flush().unwrap();
    Ok(count)
}
