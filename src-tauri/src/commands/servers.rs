use crate::engine;

#[tauri::command]
pub async fn ping_server(addr: String) -> engine::game::ping::ServerStatus {
    engine::game::ping::status(&addr).await
}

#[tauri::command]
pub async fn servers_list() -> Vec<String> {
    engine::core::settings::load().servers
}

#[tauri::command]
pub async fn servers_add(addr: String) -> Result<(), String> {
    let s = engine::core::settings::normalize_server(&addr)
        .ok_or("адрес должен быть host или host:порт")?;
    engine::core::settings::update(|st| {
        if !st.servers.contains(&s) {
            st.servers.push(s.clone());
        }
    })
}

#[tauri::command]
pub async fn servers_remove(addr: String) -> Result<(), String> {
    engine::core::settings::update(|st| st.servers.retain(|x| *x != addr))
}
