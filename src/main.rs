#![allow(dead_code)]
#![allow(unused_variables)]

mod protocol;
mod datatypes;

use std::env;
use tokio::net::TcpListener;
use protocol::{protocol_handler_main as handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();
    let port: u16 = if arguments.len() <= 1 { 25565 }
        else { arguments[1].parse().expect("failed to parse port") };
    
    let server = TcpListener::bind("127.0.0.1:".to_string() + port.to_string().as_str()).await?;
    println!("server is listening on port {}", port);

    loop {
        _ = match server.accept().await {
            Ok((socket, addr)) => handler(socket, addr),
            Err(error) => eprintln!("failed connection: {}", error)
        };
    }
}
