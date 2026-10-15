#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub fn get_hardware_id() -> Result<String, String> {
    machine_uid::get().map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub fn get_hardware_id() -> Result<String, String> {
    match machine_uid::get() {
        Ok(id) => Ok(id),
        Err(error) => ["/etc/machine-id", "/var/lib/dbus/machine-id"]
            .iter()
            .find_map(|path| std::fs::read_to_string(path).ok())
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .ok_or_else(|| error.to_string()),
    }
}
