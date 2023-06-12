use edge_gpt::{ChatSession, ConversationStyle, StreamExt};
use ezio::prelude::*;

#[tokio::main]
async fn main() {
    let mut bot = ChatSession::create(ConversationStyle::Creative, &[])
        .await
        .unwrap();

    loop {
        println!("Ask the question please:");
        let question = stdio::read_line();
        println!("Waiting for bing for response ...");
        let mut stream = bot.chat_stream(&question).await.unwrap();
        let mut msg = String::new();
        print!(">> ");
        while let Some(Ok(response)) = stream.next().await {
            if !response.text.is_empty() {
                print!("{}", response.text.trim().trim_start_matches(&msg));
                msg = response.text.trim().to_string();
            }
        }
        println!();
    }
}
