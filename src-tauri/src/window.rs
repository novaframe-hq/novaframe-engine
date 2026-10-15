use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use std::sync::atomic::Ordering;

use crate::dlog;
use crate::state::{
    COLLAPSED_HEIGHT, COLLAPSED_WIDTH, MANUAL_PAUSE, SETTINGS_PANEL_EXPANDED,
    recompute_wallpaper_visibility,
};

/// Build the system-tray icon + menu (Show Settings / Pause / Quit). Best-effort:
/// logs and returns on failure rather than aborting startup.
pub fn build_tray(app: &AppHandle) {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let show = match MenuItem::with_id(app, "tray_show", "Show Settings", true, None::<&str>) {
        Ok(i) => i,
        Err(e) => return dlog(app, &format!("[tray] item build failed: {}", e)),
    };
    let pause = match MenuItem::with_id(app, "tray_pause", "Pause Wallpaper", true, None::<&str>) {
        Ok(i) => i,
        Err(e) => return dlog(app, &format!("[tray] item build failed: {}", e)),
    };
    let quit = match MenuItem::with_id(app, "tray_quit", "Quit Novaframe", true, None::<&str>) {
        Ok(i) => i,
        Err(e) => return dlog(app, &format!("[tray] item build failed: {}", e)),
    };
    let sep = match PredefinedMenuItem::separator(app) {
        Ok(s) => s,
        Err(e) => return dlog(app, &format!("[tray] separator build failed: {}", e)),
    };
    let menu = match Menu::with_items(app, &[&show, &pause, &sep, &quit]) {
        Ok(m) => m,
        Err(e) => return dlog(app, &format!("[tray] menu build failed: {}", e)),
    };

    let pause_item = pause.clone();

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("Novaframe Engine")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "tray_show" => {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.set_focus();
                    expand_settings_panel(w);
                }
            }
            "tray_pause" => {
                let paused = !MANUAL_PAUSE.load(Ordering::Relaxed);
                MANUAL_PAUSE.store(paused, Ordering::Relaxed);
                recompute_wallpaper_visibility(app);
                let _ = pause_item.set_text(if paused {
                    "Resume Wallpaper"
                } else {
                    "Pause Wallpaper"
                });
            }
            "tray_quit" => {
                crate::state::SHUTDOWN_SIGNAL.store(true, Ordering::Relaxed);
                app.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    if let Err(e) = builder.build(app) {
        dlog(app, &format!("[tray] build failed: {}", e));
    }
}

/// Anchor the settings window flush to the monitor's right edge, vertically
/// centered, at the given LOGICAL size. Does all math in PHYSICAL pixels.
pub fn place_settings_window(
    window: &WebviewWindow,
    monitor: &tauri::window::Monitor,
    logical_width: f64,
    logical_height: f64,
) {
    let scale_factor = monitor.scale_factor();
    let target_w = (logical_width * scale_factor).round() as u32;
    let target_h = (logical_height * scale_factor).round() as u32;

    let mon_pos = monitor.position();
    let mon_size = monitor.size();

    let x = mon_pos.x + mon_size.width as i32 - target_w as i32;
    let y = mon_pos.y + (mon_size.height as i32 - target_h as i32) / 2;

    if let (Ok(ip), Ok(is)) = (window.inner_position(), window.inner_size()) {
        if ip.x == x && ip.y == y && is.width == target_w && is.height == target_h {
            return;
        }
    }

    // On Linux GTK3, tao skips gtk_window_resize() when resizable is false,
    // leaving the window stuck at GTK's 200x200 default size.
    #[cfg(target_os = "linux")]
    let _ = window.set_resizable(true);

    let _ = window.set_min_size(Some(tauri::Size::Physical(tauri::PhysicalSize::new(1, 1))));
    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
        target_w, target_h,
    )));
    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
        x, y,
    )));

    #[cfg(target_os = "linux")]
    {
        let _ = window.set_always_on_top(true);
        let _ = window.set_visible_on_all_workspaces(true);
        let _ = window.set_skip_taskbar(true);
    }

    #[cfg(target_os = "windows")]
    if let (Ok(inner), Ok(outer_pos), Ok(outer_size), Ok(inner_size)) = (
        window.inner_position(),
        window.outer_position(),
        window.outer_size(),
        window.inner_size(),
    ) {
        let dx = inner.x - outer_pos.x;
        let dy = inner.y - outer_pos.y;
        let frame_w = outer_size.width.saturating_sub(inner_size.width);
        let frame_h = outer_size.height.saturating_sub(inner_size.height);
        if dx != 0 || dy != 0 || frame_w != 0 || frame_h != 0 {
            let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                target_w + frame_w,
                target_h + frame_h,
            )));
            let _ = window.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new(x - dx, y - dy),
            ));
        }
    }
}

/// Expands the settings panel webview to its full width.
///
/// Deliberately does NOT call set_focus(). The hover-poll loops call this every
/// time the cursor crosses the right screen edge — a thing people do constantly
/// reaching for a scrollbar — and focusing here yanked focus out of whatever the
/// user was typing in. The panel still takes focus when actually clicked
/// (`acceptFirstMouse` is set on the window), and the tray "Show Settings" item
/// calls set_focus() itself before expanding, which is the one path where
/// raising the panel IS the user's explicit intent.
#[tauri::command]
pub fn expand_settings_panel(window: WebviewWindow) {
    SETTINGS_PANEL_EXPANDED.store(true, Ordering::Relaxed);
    if let Ok(Some(monitor)) = window.current_monitor() {
        place_settings_window(&window, &monitor, 360.0, 650.0);
    }
}

/// Collapses the settings panel webview to its minimal width.
#[tauri::command]
pub fn collapse_settings_panel(window: WebviewWindow) {
    SETTINGS_PANEL_EXPANDED.store(false, Ordering::Relaxed);
    if let Ok(Some(monitor)) = window.current_monitor() {
        place_settings_window(&window, &monitor, COLLAPSED_WIDTH, COLLAPSED_HEIGHT);
    }
}

const ALLOWED_STOREFRONT_HOSTS: &[&str] = &[
    "novaframe.co.uk",
    "www.novaframe.co.uk",
    "checkout.stripe.com",
    "hooks.stripe.com",
];

/// Lazily builds or focuses the storefront webview window.
#[tauri::command]
pub async fn open_storefront_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("storefront") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let url = tauri::WebviewUrl::External(
        "https://www.novaframe.co.uk/explore?source=engine"
            .parse()
            .map_err(|_| "invalid storefront url".to_string())?,
    );
    let app_handle = app.clone();
    tauri::WebviewWindowBuilder::new(&app, "storefront", url)
        .title("Novaframe Marketplace")
        .inner_size(1280.0, 800.0)
        .resizable(true)
        .decorations(true)
        .on_navigation(move |nav_url| {
            let s = nav_url.as_str();
            if s.starts_with("novaframe://") {
                crate::deeplink::process_deeplink_url(&app_handle, s);
                return false;
            }
            let host_allowed = nav_url.host_str().is_some_and(|h| {
                ALLOWED_STOREFRONT_HOSTS.contains(&h) || h.ends_with(".supabase.co")
            });
            if !host_allowed {
                let _ = app_handle.opener().open_url(s, None::<&str>);
                return false;
            }
            true
        })
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Perform window positioning pass for main and settings windows across monitors.
pub fn adjust_window_layouts(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let scale_factor = monitor.scale_factor();

            dlog(app, &format!(
                "[layout] monitor name={:?} phys_size={:?} phys_pos={:?} scale={}",
                monitor.name(), monitor.size(), monitor.position(), scale_factor
            ));

            #[cfg(target_os = "windows")]
            {
                let mon_size = *monitor.size();
                let mon_pos = *monitor.position();

                let (dx, dy, frame_w, frame_h) = match (
                    window.inner_position(),
                    window.outer_position(),
                    window.outer_size(),
                    window.inner_size(),
                ) {
                    (Ok(inner), Ok(outer_pos), Ok(outer_size), Ok(inner_size)) => (
                        inner.x - outer_pos.x,
                        inner.y - outer_pos.y,
                        outer_size.width.saturating_sub(inner_size.width),
                        outer_size.height.saturating_sub(inner_size.height),
                    ),
                    _ => (0, 0, 0, 0),
                };
                let want_w = mon_size.width + frame_w;
                let want_h = mon_size.height + frame_h;
                let want_x = mon_pos.x - dx;
                let want_y = mon_pos.y - dy;

                let already_placed = match (window.outer_position(), window.outer_size()) {
                    (Ok(p), Ok(s)) => {
                        p.x == want_x && p.y == want_y && s.width == want_w && s.height == want_h
                    }
                    _ => false,
                };
                if !already_placed {
                    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                        want_w, want_h,
                    )));
                    let _ = window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(want_x, want_y),
                    ));
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                let logical_size = monitor.size().to_logical::<f64>(scale_factor);
                let logical_pos = monitor.position().to_logical::<f64>(scale_factor);

                // Added idempotence check (MiniMax Audit Finding D) to eliminate repaint churn on macOS.
                // On Linux X11, a top panel strut (e.g. 27px bar) can offset y or clamp height slightly.
                let y_slack = if cfg!(target_os = "linux") { 64.0 } else { 1.0 };
                let already_placed = match (window.outer_position(), window.outer_size()) {
                    (Ok(p), Ok(s)) => {
                        let current_pos = p.to_logical::<f64>(scale_factor);
                        let current_size = s.to_logical::<f64>(scale_factor);
                        (current_pos.x - logical_pos.x).abs() < 1.0 &&
                        (current_pos.y - logical_pos.y).abs() <= y_slack &&
                        (current_size.width - logical_size.width).abs() < 1.0 &&
                        (current_size.height - logical_size.height).abs() <= y_slack
                    }
                    _ => false,
                };

                if !already_placed {
                    let _ = window.set_size(tauri::Size::Logical(logical_size));
                    let _ = window.set_position(tauri::Position::Logical(logical_pos));
                }
            }

            // Reassert Linux EWMH hints and click-through input region after
            // geometry changes: GTK/WMs can clear keep-below, workspace, or
            // input-shape state while resizing the wallpaper.
            #[cfg(target_os = "linux")]
            {
                let _ = window.set_always_on_bottom(true);
                let _ = window.set_visible_on_all_workspaces(true);
                let _ = window.set_skip_taskbar(true);
                let _ = window.set_focusable(false);
                let _ = window.set_ignore_cursor_events(true);
            }

            if let Some(settings_window) = app.get_webview_window("settings") {
                #[cfg(target_os = "linux")]
                let _ = settings_window.set_visible_on_all_workspaces(true);

                let expanded = SETTINGS_PANEL_EXPANDED.load(Ordering::Relaxed);
                let target_width = if expanded { 360.0 } else { COLLAPSED_WIDTH };
                let target_height = if expanded { 650.0 } else { COLLAPSED_HEIGHT };
                place_settings_window(&settings_window, &monitor, target_width, target_height);
            }
        }
    }
}
