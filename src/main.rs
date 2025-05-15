use client::Client;

mod proto;
mod client;
mod socket;
mod constant;
mod util;

#[tokio::main]
async fn main() {
    let client = Client::default();
    client.connect().await; 
    loop {}
}