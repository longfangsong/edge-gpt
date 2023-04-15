use std::fs::File;

use edge_gpt::{ChatSession, ConversationStyle, CookieInFile};
use ezio::prelude::*;

#[tokio::main]
async fn main() {
    let file = File::open("./cookies.json")
        .map(std::io::BufReader::new)
        .unwrap();
    let cookies: Vec<CookieInFile> = serde_json::from_reader(file).unwrap();
    let mut bot = ChatSession::create(ConversationStyle::Balanced, &cookies)
        .await
        .unwrap();
    println!("Ask the question please:");
    let question = stdio::read_line();
    println!("Waiting for bing for response ...");
    let response = bot.send_message(&question).await.unwrap();
    println!(">> {}", response.text);
    for (i, source_attribution) in response.source_attributions.iter().enumerate() {
        println!("[{}]: {}", i + 1, source_attribution);
    }
}
