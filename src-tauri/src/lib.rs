mod api;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            api::init().map_err(std::io::Error::other)?;
            // Bind before showing the window, so the first frontend API call
            // cannot race the loopback server's startup.
            let (listener, router, port) =
                tauri::async_runtime::block_on(api::start()).map_err(std::io::Error::other)?;
            app.manage(ApiPort(port));
            tauri::async_runtime::spawn(async {
                if let Err(error) = api::serve(listener, router).await {
                    log::error!("Opendots local API stopped: {error}");
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![api_port])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct ApiPort(u16);

#[tauri::command]
fn api_port(port: tauri::State<'_, ApiPort>) -> u16 {
    port.0
}

