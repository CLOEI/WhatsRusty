use prost::Message;
use rand_core::{OsRng, RngCore};
use crate::proto::whatsapp::client_payload::{user_agent, web_info, DevicePairingRegistrationData, UserAgent, WebInfo};
use crate::proto::whatsapp::{client_payload, device_props, ClientPayload, DeviceProps};
use crate::proto::whatsapp::client_payload::user_agent::AppVersion;
use crate::utils::key::{Key, PreKey};

pub struct Device {
    pub noise_key: Key,
    pub identity_key: Key,
    pub signed_pre_key: PreKey,
    pub registration_id: u32,
    pub adv_secret_key: [u8; 32]
}

impl Device {
    pub fn new() -> Self {
        let mut random_byte = [0u8; 32];
        OsRng.fill_bytes(&mut random_byte);
        let identity_key = Key::new();
        let signed_pre_key = identity_key.create_signed_pre_key(1);

        Self {
            noise_key: Key::new(),
            identity_key,
            signed_pre_key,
            registration_id: OsRng.next_u32(),
            adv_secret_key: random_byte
        }
    }

    pub fn create_register_payload(&self) -> ClientPayload {
        let reg_id: [u8; 4] = self.registration_id.to_be_bytes();
        let pre_key_id: [u8; 4] = self.signed_pre_key.id.to_be_bytes();

        ClientPayload {
            user_agent: Some(UserAgent {
                platform: Some(user_agent::Platform::Web.into()),
                release_channel: Some(user_agent::ReleaseChannel::Release.into()),
                app_version: Some(AppVersion {
                    primary: Some(2),
                    secondary: Some(3000),
                    tertiary: Some(1022419966),
                    ..Default::default()
                }),
                mcc: Some("000".to_string()),
                mnc: Some("000".to_string()),
                os_version: Some("0.1.0".to_string()),
                manufacturer: Some("".to_string()),
                device: Some("Desktop".to_string()),
                os_build_number: Some("0.1.0".to_string()),
                locale_language_iso6391: Some("en".to_string()),
                locale_country_iso31661_alpha2: Some("en".to_string()),
                ..Default::default()
            }),
            web_info: Some(WebInfo {
                web_sub_platform: Some(web_info::WebSubPlatform::WebBrowser.into()),
                ..Default::default()
            }),
            connect_type: Some(client_payload::ConnectType::WifiUnknown.into()),
            connect_reason: Some(client_payload::ConnectReason::UserActivated.into()),
            device_pairing_data: Some(DevicePairingRegistrationData {
                e_regid: Some(reg_id.to_vec()),
                e_keytype: Some(vec![0x05]),
                e_ident: Some(self.identity_key.public.as_bytes().to_vec()),
                e_skey_id: Some(pre_key_id[1..].to_vec()),
                e_skey_val: Some(self.signed_pre_key.key.public.as_bytes().to_vec()),
                e_skey_sig: Some(self.signed_pre_key.signature.to_vec()),
                build_hash: Some(calculate_wa_version_hash().to_vec()),
                device_props: Some(DeviceProps {
                    os: Some("WhatsRusty".to_string()),
                    version: Some(device_props::AppVersion {
                        primary: Some(0),
                        secondary: Some(1),
                        tertiary: Some(0),
                        ..Default::default()
                    }),
                    platform_type: Some(0),
                    require_full_sync: Some(false),
                    ..Default::default()
                }.encode_to_vec()),
                ..Default::default()
            }),
            passive: Some(false),
            pull: Some(false),
            ..Default::default()
        }
    }
}

fn calculate_wa_version_hash() -> [u8; 16] {
    let version = "2.3000.1022419966";
    let digest = md5::compute(version.as_bytes());
    digest.into()
}