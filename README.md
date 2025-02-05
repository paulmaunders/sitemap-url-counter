# Sitemap URL Counter

A fast command-line tool to count URLs in XML sitemaps, including sitemap indexes.

## Prerequisites

- Rust and Cargo (install from https://rustup.rs/)

## Installation

```bash
git clone [your-repo-url]
cd sitemap-url-counter
cargo build --release
```

## Usage

```bash
./target/release/sitemap-url-counter <sitemap_url> [--debug]
```

The optional `--debug` flag can be used to display detailed debug information during execution.

Example:
```bash
./target/release/sitemap-url-counter https://example.com/sitemap.xml
```

The tool will:
1. Download the main sitemap
2. Process any sub-sitemaps if it's a sitemap index
3. Count URLs in all sitemaps
4. Display progress and results

## Output

The tool shows:
- Progress bar during processing
- Individual URL counts per sitemap
- Total URL count across all sitemaps
