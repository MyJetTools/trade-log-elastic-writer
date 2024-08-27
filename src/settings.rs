use serde::{Deserialize, Serialize};
use service_sdk::async_trait;

service_sdk::macros::use_settings!();
#[derive(
    my_settings_reader::SettingsModel,
    AutoGenerateSettingsTraits,
    SdkSettingsTraits,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
pub struct SettingsModel {
    pub my_sb_tcp_host_port: String,
    pub my_telemetry: String,
    pub seq_conn_string: String,
    pub env_source: String,
    pub esecure: String,
    pub elastic_url: String,
}
