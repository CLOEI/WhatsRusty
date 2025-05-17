use std::sync::Arc;

use ezsockets::ClientConfig;

use crate::{constant, device::Device, socket::{frame_socket::FrameSocket}};

pub struct Client {
    pub device: Device,
}

impl Client {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            device: Device::new(),
        })
    }

    pub async fn connect(self: &Arc<Self>)  {
        let client = Arc::clone(self);
        let config = ClientConfig::new(constant::WS_URL).header("Origin", constant::ORIGIN);
        let (_, future) = ezsockets::connect(move |handle| {
            FrameSocket::new(handle, Arc::clone(&client))
        }, config).await;
        tokio::spawn(async move {
            future.await.unwrap();
        });
    }
}