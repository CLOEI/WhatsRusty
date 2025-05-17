
#[derive(Debug, Clone)]
pub struct JID {
    pub user: Option<String>,
    pub raw_agent: Option<u8>,
    pub device: Option<u16>,
    pub integrator: Option<u16>,
    pub server: Option<String>,
}

impl JID {
    pub fn new(user: Option<String>, raw_agent: Option<u8>, device: Option<u16>, integrator: Option<u16>, server: Option<String>) -> Self {
        Self { user, raw_agent, device, integrator, server }
    }
}