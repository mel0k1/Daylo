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
