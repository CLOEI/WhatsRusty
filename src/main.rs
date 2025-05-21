use crate::client::{connect, Events};

mod proto;
mod constant;
mod client;
mod utils;
mod socket;
mod device;
mod types;

struct MyClient {}

impl Events for MyClient {
    fn on_qr(&self, qr: &str) {
        println!("QR Code: {}", qr);
    }
}

#[tokio::main]
async fn main() {
    let client = connect(MyClient{}).await;

    loop {}
}
