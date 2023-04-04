use std::{fs::File, path::PathBuf};

use clap::Parser;
use edge_gpt::{ChatSession, CookieInFile};
use ezio::prelude::*;

/// Chat with new Bing continually
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// auto inc major
    #[arg(long, group = "input")]
    create: Option<PathBuf>,

    #[arg(long, group = "input")]
    load: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Some(target_path) = args.create {
        let file = File::open("./cookies.json")
            .map(std::io::BufReader::new)
            .unwrap();
        let cookies: Vec<CookieInFile> = serde_json::from_reader(file).unwrap();
        let mut bot = ChatSession::create(&cookies).await;
        println!("Ask the question please:");
        let question = stdio::read_line();
        println!("Waiting for bing for response ...");
        let response = bot.send_message(&question).await.unwrap();
        println!(">> {}", response.text);
        for (i, source_attribution) in response.source_attributions.iter().enumerate() {
            println!("[{}]: {}", i + 1, source_attribution);
        }
        let file = File::create(&target_path).unwrap();
        serde_json::to_writer(file, &bot).unwrap();
    } else if let Some(source_path) = args.load {
        let file = File::open(&source_path).unwrap();
        let mut bot: ChatSession = serde_json::from_reader(file).unwrap();
        println!("Ask the question please:");
        let question = stdio::read_line();
        println!("Waiting for bing for response ...");
        let response = bot.send_message(&question).await.unwrap();
        println!(">> {}", response.text);
        for (i, source_attribution) in response.source_attributions.iter().enumerate() {
            println!("[{}]: {}", i + 1, source_attribution);
        }
        let file = File::create(&source_path).unwrap();
        serde_json::to_writer(file, &bot).unwrap();
    }
}
