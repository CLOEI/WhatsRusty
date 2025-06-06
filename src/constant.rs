use std::time::Duration;

pub const WS_URL: &str = "wss://web.whatsapp.com/ws/chat";
pub const ORIGIN: &str = "https://web.whatsapp.com";
pub const CONN_HEADER: [u8; 4] = [b'W', b'A', 6, 3]; // 6 and 3 not sure what it is
pub const NOISE_PATTERN: &str = "Noise_XX_25519_AESGCM_SHA256\x00\x00\x00\x00";
pub const FRAME_MAX_SIZE: usize = 2 << 23;
pub const FRAME_LENGTH_SIZE: usize = 3;

pub const KEEPALIVE_RESPONSE_DEADLINE: Duration = Duration::from_secs(10);
pub const KEEPALIVE_INTERVAL_MIN: Duration = Duration::from_secs(20);
pub const KEEPALIVE_INTERVAL_MAX: Duration = Duration::from_secs(30);
pub const KEEPALIVE_MAX_FAIL_TIME: Duration = Duration::from_secs(180);
