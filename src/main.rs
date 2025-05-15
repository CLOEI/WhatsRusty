use client::Client;

mod proto;
mod client;
mod socket;
mod constant;
mod util;
mod device;

#[tokio::main]
async fn main() {
    let client = Client::new();
    client.connect().await; 
    loop {}
}