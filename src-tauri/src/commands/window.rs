use serde::Deserialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Deserialize, Clone)]
pub struct InputRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn trace(msg: &str) {
    use std::io::Write;
    if std::env::var_os("AETHER_DEBUG").is_none() {
        return;
    }
    let path = std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".cache/aether-debug.log"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/aether-debug.log"));
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", msg);
    }
}

#[tauri::command]
pub fn debug_log(msg: String) {
    trace(&msg);
}

#[tauri::command]
pub fn set_input_shape(app: AppHandle, rects: Vec<InputRect>) -> Result<(), String> {
    trace(&format!("set_input_shape {:?}", rects));
    #[cfg(target_os = "linux")]
    {
        use gtk::cairo::{RectangleInt, Region};
        use gtk::prelude::*;

        let Some(window) = app.get_webview_window("aether-widget") else {
            return Err("window not found".to_string());
        };

        let Ok(gtk_win) = window.gtk_window() else {
            return Err("failed to get gtk window".to_string());
        };

        if rects.is_empty() {
            gtk_win.input_shape_combine_region(None);
            return Ok(());
        }

        let cairo_rects: Vec<RectangleInt> = rects
            .iter()
            .map(|r| {
                RectangleInt::new(
                    r.x,
                    r.y,
                    r.width.max(1),
                    r.height.max(1),
                )
            })
            .collect();

        let region = Region::create_rectangles(&cairo_rects);
        gtk_win.input_shape_combine_region(Some(&region));

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, rects);
        Ok(())
    }
}

#[tauri::command]
pub fn set_desktop_margin(margin: u32) -> Result<(), String> {
    trace(&format!("set_desktop_margin {}", margin));
    #[cfg(target_os = "linux")]
    {
        let Some(home) = dirs::home_dir() else {
            return Err("failed to get home directory".to_string());
        };
        let rc_path = home.join(".config/labwc/rc.xml");
        if !rc_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&rc_path)
            .map_err(|e| format!("failed to read rc.xml: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut margin_handled = false;

        for line in lines {
            if line.trim().starts_with("<margin") {
                if margin > 0 && !margin_handled {
                    new_lines.push(format!("  <margin top=\"{}\" />", margin));
                    margin_handled = true;
                }
                // If margin == 0, omit line to remove it
            } else {
                new_lines.push(line.to_string());
                if margin > 0 && !margin_handled && line.contains("<labwc_config") {
                    new_lines.push(format!("  <margin top=\"{}\" />", margin));
                    margin_handled = true;
                }
            }
        }

        let new_content = new_lines.join("\n") + "\n";
        if new_content != content {
            std::fs::write(&rc_path, &new_content)
                .map_err(|e| format!("failed to write rc.xml: {}", e))?;

            let _ = std::process::Command::new("labwc")
                .arg("-r")
                .spawn();
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = margin;
        Ok(())
    }
}

