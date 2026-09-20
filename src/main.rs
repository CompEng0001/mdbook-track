use anyhow::{Context, Result};
use mdbook_preprocessor::Preprocessor;
use mdbook_track::TrackPreprocessor;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process;

const TRACK_JS: &str = include_str!("../assets/mdbook-track.js");
const TRACK_CSS: &str = include_str!("../assets/mdbook-track.css");

fn main() {
    if let Err(error) = run() {
        eprintln!("mdbook-track: {error:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        Some("supports") => {
            let renderer = args.next().context("missing renderer name")?;
            let supported = TrackPreprocessor::new().supports_renderer(&renderer)?;
            process::exit(if supported { 0 } else { 1 });
        }
        Some("init") => {
            let root = args.next().unwrap_or_else(|| ".".to_owned());
            initialise_assets(Path::new(&root))?;
            Ok(())
        }
        Some("--help") | Some("-h") | Some("help") => {
            print_help();
            Ok(())
        }
        Some(other) => anyhow::bail!("unknown argument: {other}"),
        None => handle_preprocessing(),
    }
}

fn handle_preprocessing() -> Result<()> {
    let preprocessor = TrackPreprocessor::new();
    let (ctx, book) = mdbook_preprocessor::parse_input(io::stdin())?;
    let processed = preprocessor.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed)?;
    Ok(())
}

fn initialise_assets(root: &Path) -> Result<()> {
    if !root.join("book.toml").exists() {
        anyhow::bail!("{} does not contain book.toml", root.display());
    }

    fs::write(root.join("mdbook-track.js"), TRACK_JS)
        .context("failed to write mdbook-track.js")?;
    fs::write(root.join("mdbook-track.css"), TRACK_CSS)
        .context("failed to write mdbook-track.css")?;

    println!("Installed mdbook-track.js and mdbook-track.css in {}", root.display());
    println!();
    println!("Add the following to book.toml:");
    println!();
    println!("[preprocessor.track]");
    println!("book-id = \"your-workbook-id\"");
    println!("renderers = [\"html\"]");
    println!();
    println!("[output.html]");
    println!("additional-js = [\"mdbook-track.js\"]");
    println!("additional-css = [\"mdbook-track.css\"]");

    Ok(())
}

fn print_help() {
    println!("mdbook-track {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("  mdbook-track                 Run as an mdBook preprocessor");
    println!("  mdbook-track supports html   Check renderer support");
    println!("  mdbook-track init [BOOK]     Install JS/CSS assets into a book");
}
