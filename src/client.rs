use std::sync::Arc;

use ezsockets::ClientConfig;
use paris::info;

use crate::{constant, device::Device, socket::frame_socket::FrameSocket, util::binary::{Node, Value}};

pub struct Client {
    pub device: Device,
    pub handle: Option<Box<dyn Events + Send + 'static>>,
}

impl Client {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            device: Device::new(),
            handle: None,
        })
    }
}

pub async fn connect<E: Events + 'static>(client: E) -> Arc<Client> {
    info!("Dialing {}", constant::WS_URL);

    let client = Arc::new(Client {
        device: Device::new(),
        handle: Some(Box::new(client)),
    });

    let config = ClientConfig::new(constant::WS_URL).header("Origin", constant::ORIGIN);
    let client_clone = client.clone();
    let (_, future) = ezsockets::connect(move |handle| {
        FrameSocket::new(handle, client_clone.clone())
    }, config).await;

    tokio::spawn(async move {
        future.await.unwrap();
    });

    client
}

pub trait Events: Send + Sync {
    fn on_qr(&self, qr: String);
}