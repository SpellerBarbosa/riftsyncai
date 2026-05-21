use tauri::Manager;

#[tauri::command]
pub(crate) async fn get_monitor_dimensions(app: tauri::AppHandle) -> Result<(u32, u32), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Primary monitor not found")?;
    let size = monitor.size();
    Ok((size.width, size.height))
}

#[tauri::command]
pub(crate) async fn resize_main_window(app: tauri::AppHandle, expanded: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let (width, height) = if expanded {
            (400.0, 100.0)
        } else {
            (400.0, 50.0)
        };
        window.set_resizable(true).map_err(|e| e.to_string())?;
        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
