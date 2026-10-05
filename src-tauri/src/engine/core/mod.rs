pub mod http;
pub mod paths;

use serde::Serialize;

#[derive(Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

pub fn app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub use http::{client, download, json, sha1_of, text};
pub use paths::{data_dir, game_root, instance_dir, java_dir, version_dir};
