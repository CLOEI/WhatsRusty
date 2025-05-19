use client::{connect, Client, Events};

mod proto;
mod client;
mod socket;
mod constant;
mod util;
mod device;
mod r#type;

struct MyClient;
impl Events for MyClient {
    fn on_qr(&self, qr: String) {
        println!("QR: {}", qr);
    }
}

#[tokio::main]
async fn main() {
    let client = connect(MyClient).await;
    loop {}
}