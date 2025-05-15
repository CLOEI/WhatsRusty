use ezsockets::ClientConfig;

use crate::{constant, socket::{frame_socket::FrameSocket, noise_socket::NoiseSocket}};

#[derive(Default)]
pub struct Client {
    ns: Option<NoiseSocket>,
}

impl Client {
    pub async fn connect(&self)  {
        let config = ClientConfig::new(constant::WS_URL).header("Origin", constant::ORIGIN);
        let (_, future) = ezsockets::connect(|handle| FrameSocket {
            handle: Some(handle),
            ..Default::default()
        }, config).await;
        tokio::spawn(async move {
            future.await.unwrap();
        });
    }
}