use std::sync::atomic::Ordering;
use crate::dlog;
#[cfg(any(target_os = "linux", test))]
use std::path::Path;
use crate::state::{
    BATTERY_SAVER_ENABLED, FULLSCREEN_ACTIVE, MANUAL_PAUSE, ON_BATTERY, WINDOW_OCCLUDED,
    recompute_wallpaper_visibility,
};

/// True when the machine is running on battery rather than mains power.
///
/// macOS: `pmset -g batt` prints "Now drawing from 'AC Power'" or "'Battery
/// Power'". Shelling out once a minute is cheaper in both code and binary size
/// than binding IOKit's power-source APIs for one boolean.
#[cfg(target_os = "macos")]
pub fn running_on_battery() -> bool {
    std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("Battery Power"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn running_on_battery() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    unsafe {
        let mut status = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut status).is_ok() {
            // ACLineStatus: 0 = offline (battery), 1 = online, 255 = unknown.
            status.ACLineStatus == 0
        } else {
            false
        }
    }
}

#[cfg(target_os = "linux")]
pub fn running_on_battery() -> bool {
    linux_running_on_battery(Path::new("/sys/class/power_supply"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn running_on_battery() -> bool {
    false
}

#[cfg(any(target_os = "linux", test))]
fn linux_running_on_battery(power_supply_dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(power_supply_dir) else {
        return false;
    };

    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        let is_device_scope = std::fs::read_to_string(path.join("scope"))
            .ok()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("Device"));
        if is_device_scope {
            return false;
        }
        let is_battery = std::fs::read_to_string(path.join("type"))
            .ok()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("Battery"));
        is_battery
            && std::fs::read_to_string(path.join("status"))
                .ok()
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("Discharging"))
    })
}

/// Turn "pause on battery" on or off from the settings panel. Recomputes
/// immediately so switching it on while already unplugged pauses now, rather
/// than at the next poll.
#[tauri::command]
pub fn set_battery_saver(app: tauri::AppHandle, enabled: bool) {
    BATTERY_SAVER_ENABLED.store(enabled, Ordering::Relaxed);
    recompute_wallpaper_visibility(&app);
    dlog(&app, &format!("[pause] battery saver enabled={}", enabled));
}

/// Current pause state, for the frontend to QUERY rather than wait to be told.
#[tauri::command]
pub fn get_wallpaper_paused() -> bool {
    MANUAL_PAUSE.load(Ordering::Relaxed)
        || FULLSCREEN_ACTIVE.load(Ordering::Relaxed)
        || WINDOW_OCCLUDED.load(Ordering::Relaxed)
        || (BATTERY_SAVER_ENABLED.load(Ordering::Relaxed) && ON_BATTERY.load(Ordering::Relaxed))
}

/// Spawn the background battery source polling thread (polls every 60 seconds).
pub fn spawn_battery_monitor(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last = running_on_battery();
        ON_BATTERY.store(last, Ordering::Relaxed);
        dlog(&app, &format!("[pause] power source at startup: on battery={}", last));
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let on_battery = running_on_battery();
            if on_battery != last {
                last = on_battery;
                ON_BATTERY.store(on_battery, Ordering::Relaxed);
                recompute_wallpaper_visibility(&app);
                dlog(&app, &format!("[pause] on battery={}", on_battery));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::linux_running_on_battery;
    use std::fs;
    use std::path::Path;

    struct TempSupplyDir(std::path::PathBuf);

    impl TempSupplyDir {
        fn new() -> Self {
            static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "novaframe-power-test-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempSupplyDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_supply(root: &Path, name: &str, supply_type: &str, status: &str) {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("type"), supply_type).unwrap();
        fs::write(path.join("status"), status).unwrap();
    }

    #[test]
    fn detects_discharging_battery_case_insensitively() {
        let temp = TempSupplyDir::new();
        write_supply(temp.path(), "BAT0", " bAtTeRy\n", " dIsChArGiNg\n");
        assert!(linux_running_on_battery(temp.path()));
    }

    #[test]
    fn returns_false_for_ac_or_desktop_supplies() {
        let temp = TempSupplyDir::new();
        write_supply(temp.path(), "AC", "Mains", "Discharging");
        write_supply(temp.path(), "BAT0", "Battery", "Charging");
        assert!(!linux_running_on_battery(temp.path()));

        write_supply(temp.path(), "BAT1", "Battery", "Discharging");
        assert!(linux_running_on_battery(temp.path()));
    }

    #[test]
    fn ignores_peripheral_device_batteries() {
        let temp = TempSupplyDir::new();
        write_supply(temp.path(), "hid-mouse-battery", "Battery", "Discharging");
        fs::write(temp.path().join("hid-mouse-battery").join("scope"), "Device\n").unwrap();
        assert!(!linux_running_on_battery(temp.path()));

        fs::write(temp.path().join("hid-mouse-battery").join("scope"), "System\n").unwrap();
        assert!(linux_running_on_battery(temp.path()));
    }

    #[test]
    fn returns_false_for_missing_directory_or_unreadable_fields() {
        let temp = TempSupplyDir::new();
        assert!(!linux_running_on_battery(&temp.path().join("missing")));
        fs::create_dir(temp.path().join("BAT0")).unwrap();
        assert!(!linux_running_on_battery(temp.path()));
    }
}
