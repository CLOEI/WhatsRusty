use client::Client;

mod proto;
mod client;
mod socket;
mod constant;
mod util;
mod device;
mod r#type;

#[tokio::main]
async fn main() {
    let client = Client::new();
    client.connect().await; 
    loop {}
}