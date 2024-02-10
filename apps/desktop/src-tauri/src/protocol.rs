use log::info;
use tauri::{
    plugin::{self, Plugin},
    AppHandle, Runtime,
};

pub struct ProtocolPlugin {}

impl Default for ProtocolPlugin {
    fn default() -> Self {
        Self {}
    }
}

impl<R: Runtime> Plugin<R> for ProtocolPlugin {
    fn name(&self) -> &'static str {
        "protocol"
    }

    fn initialize(&mut self, app: &AppHandle<R>, config: serde_json::Value) -> plugin::Result<()> {
        info!("Initializing protocol plugin");
        tauri::async_runtime::spawn(async move {});
        Ok(())
    }
}
