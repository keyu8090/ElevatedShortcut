mod model;
mod icon;
mod lnk;
mod elevate;
mod storage;
mod tasks;
mod util;
mod win_shortcut;
mod autostart;

use std::{path::PathBuf, time::SystemTime};

use base64::Engine;
use model::ProgramEntry;
use tauri::Manager;

const AUTOSTART_VALUE_NAME: &str = "SkipUAC";

fn icon_data_url(icon_path: &str) -> Option<String> {
    let bytes = std::fs::read(icon_path).ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{b64}"))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn sanitize_for_task_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "App".to_string()
    } else {
        trimmed
    }
}

fn task_name_for(name: &str, target: &str, args: Option<&str>) -> String {
    let mut h = blake3::Hasher::new();
    h.update(target.as_bytes());
    if let Some(a) = args {
        h.update(b"\n");
        h.update(a.as_bytes());
    }
    let hash = h.finalize().to_hex();
    let short = &hash.as_str()[..10];
    format!("elevated_{}_{}", sanitize_for_task_name(name), short)
}

pub fn run_task_cli(task_name: &str) -> Result<(), String> {
    tasks::run_task(task_name)
}

pub fn install_pending_cli(pending_path: &str) -> Result<usize, String> {
    elevate::perform_pending_install(std::path::Path::new(pending_path))
}

pub fn cleanup_pending_cli(pending_path: &str) -> Result<usize, String> {
    elevate::perform_pending_cleanup(std::path::Path::new(pending_path))
}

#[tauri::command]
fn get_programs(app: tauri::AppHandle) -> Result<Vec<ProgramEntry>, String> {
    let paths = storage::resolve_paths(&app)?;
    storage::ensure_dirs(&paths)?;
    let mut programs = storage::load_programs(&paths)?;
    let mut changed = false;
    for p in &mut programs {
        if !p.installed {
            if let Some(sp) = &p.desktop_shortcut_path {
                if std::path::Path::new(sp).exists() {
                    p.installed = true;
                    changed = true;
                }
            }
        }
    }
    if changed {
        storage::save_programs(&paths, &programs)?;
    }
    for p in &mut programs {
        p.icon_data_url = icon_data_url(&p.icon_path);
    }
    Ok(programs)
}

#[tauri::command]
fn reorder_programs(app: tauri::AppHandle, ids: Vec<String>) -> Result<(), String> {
    let paths = storage::resolve_paths(&app)?;
    storage::ensure_dirs(&paths)?;

    let programs = storage::load_programs(&paths)?;
    let mut by_id: std::collections::HashMap<String, ProgramEntry> =
        programs.into_iter().map(|p| (p.id.clone(), p)).collect();

    let mut out: Vec<ProgramEntry> = Vec::with_capacity(by_id.len());
    for id in &ids {
        if let Some(p) = by_id.remove(id) {
            out.push(p);
        }
    }

    // Append any remaining entries (unknown ids or older items) to keep data intact.
    for (_, p) in by_id {
        out.push(p);
    }

    storage::save_programs(&paths, &out)?;
    Ok(())
}

#[tauri::command]
fn add_program_from_path(app: tauri::AppHandle, path: String) -> Result<ProgramEntry, String> {
    let paths = storage::resolve_paths(&app)?;
    storage::ensure_dirs(&paths)?;

    let source_path = PathBuf::from(path);
    if !source_path.exists() {
        return Err("path does not exist".to_string());
    }

    let source_ext = source_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let (target_path, arguments, mut icon_source) = if source_ext == "lnk" {
        let r = lnk::resolve_lnk(&source_path)?;
        let args = r.arguments.clone().unwrap_or_default().to_ascii_lowercase();
        if args.contains("--run-task") && args.contains("elevated_") {
            return Err("You dropped an (elevated).lnk. Please drop the original shortcut (.lnk) that points to the target app.".to_string());
        }
        let icon_source = r.icon_path.clone().unwrap_or_else(|| r.target_path.clone());
        (r.target_path, r.arguments, icon_source)
    } else {
        (source_path.clone(), None, source_path.clone())
    };

    if !icon_source.exists() {
        icon_source = target_path.clone();
    }

    if !target_path.exists() {
        return Err("target path does not exist".to_string());
    }

    let target_ext = target_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if target_ext != "exe" {
        return Err("only .exe targets are supported".to_string());
    }

    let name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("App")
        .to_string();

    let args_str = arguments.as_deref();
    let task_name = task_name_for(&name, &target_path.to_string_lossy(), args_str);

    let mut programs = storage::load_programs(&paths)?;
    if let Some(existing_idx) = programs.iter().position(|p| p.task_name == task_name) {
        let mut existing = programs[existing_idx].clone();
        existing.icon_data_url = icon_data_url(&existing.icon_path);

        let desktop = app
            .path()
            .desktop_dir()
            .map_err(|e| format!("resolve desktop dir failed: {e}"))?;
        let should_create_shortcut = existing
            .desktop_shortcut_path
            .as_deref()
            .map(|p| !std::path::Path::new(p).exists())
            .unwrap_or(true);
        if should_create_shortcut {
            let icon_path = target_path.to_string_lossy().to_string();
            let shortcut = win_shortcut::create_shortcut_in_dir(&desktop, &existing.name, &existing.task_name, &icon_path)?;
            programs[existing_idx].desktop_shortcut_path = Some(shortcut.clone());
            storage::save_programs(&paths, &programs)?;
            existing.desktop_shortcut_path = Some(shortcut);
        }

        return Ok(existing);
    }

    let target_path_str = target_path.to_string_lossy();
    tasks::create_task(&task_name, target_path_str.as_ref(), args_str)?;

    let mut icon_hasher = blake3::Hasher::new();
    icon_hasher.update(target_path.to_string_lossy().as_bytes());
    if let Some(a) = args_str {
        icon_hasher.update(b"\n");
        icon_hasher.update(a.as_bytes());
    }
    let icon_hash = icon_hasher.finalize().to_hex();
    let icon_png_path = paths.icons_dir.join(format!("{}.png", &icon_hash.as_str()[..16]));
    if !icon_png_path.exists() {
        icon::extract_icon_png_for_path(&icon_source, &icon_png_path)?;
    }

    let entry = ProgramEntry {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        source_path: source_path.to_string_lossy().to_string(),
        target_path: target_path.to_string_lossy().to_string(),
        arguments,
        icon_path: icon_png_path.to_string_lossy().to_string(),
        icon_data_url: icon_data_url(&icon_png_path.to_string_lossy()),
        desktop_shortcut_path: None,
        installed: true,
        task_name,
        created_at_unix_ms: now_unix_ms(),
    };

    let desktop = app
        .path()
        .desktop_dir()
        .map_err(|e| format!("resolve desktop dir failed: {e}"))?;
    let target_icon_path = target_path.to_string_lossy().to_string();
    let shortcut = win_shortcut::create_shortcut_in_dir(&desktop, &entry.name, &entry.task_name, &target_icon_path)?;

    let mut saved_entry = entry.clone();
    saved_entry.desktop_shortcut_path = Some(shortcut.clone());
    programs.push(saved_entry);
    storage::save_programs(&paths, &programs)?;

    let mut returned = entry;
    returned.desktop_shortcut_path = Some(shortcut);

    Ok(returned)
}

#[tauri::command]
fn add_program_draft_from_path(app: tauri::AppHandle, path: String) -> Result<ProgramEntry, String> {
    let paths = storage::resolve_paths(&app)?;
    storage::ensure_dirs(&paths)?;

    let source_path = PathBuf::from(path);
    if !source_path.exists() {
        return Err("path does not exist".to_string());
    }

    let source_ext = source_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let (target_path, arguments, mut icon_source) = if source_ext == "lnk" {
        let r = lnk::resolve_lnk(&source_path)?;
        let args = r.arguments.clone().unwrap_or_default().to_ascii_lowercase();
        if args.contains("--run-task") && args.contains("elevated_") {
            return Err("You dropped an (elevated).lnk. Please drop the original shortcut (.lnk) that points to the target app.".to_string());
        }
        let icon_source = r.icon_path.clone().unwrap_or_else(|| r.target_path.clone());
        (r.target_path, r.arguments, icon_source)
    } else {
        (source_path.clone(), None, source_path.clone())
    };

    if !icon_source.exists() {
        icon_source = target_path.clone();
    }

    if !target_path.exists() {
        return Err("target path does not exist".to_string());
    }

    let target_ext = target_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if target_ext != "exe" {
        return Err("only .exe targets are supported".to_string());
    }

    let name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("App")
        .to_string();

    let args_str = arguments.as_deref();
    let task_name = task_name_for(&name, &target_path.to_string_lossy(), args_str);

    let mut programs = storage::load_programs(&paths)?;
    if let Some(existing) = programs.iter().find(|p| p.task_name == task_name) {
        let mut p = existing.clone();
        p.icon_data_url = icon_data_url(&p.icon_path);
        return Ok(p);
    }

    let mut icon_hasher = blake3::Hasher::new();
    icon_hasher.update(target_path.to_string_lossy().as_bytes());
    if let Some(a) = args_str {
        icon_hasher.update(b"\n");
        icon_hasher.update(a.as_bytes());
    }
    let icon_hash = icon_hasher.finalize().to_hex();
    let icon_png_path = paths.icons_dir.join(format!("{}.png", &icon_hash.as_str()[..16]));
    if !icon_png_path.exists() {
        icon::extract_icon_png_for_path(&icon_source, &icon_png_path)?;
    }

    let entry = ProgramEntry {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        source_path: source_path.to_string_lossy().to_string(),
        target_path: target_path.to_string_lossy().to_string(),
        arguments,
        icon_path: icon_png_path.to_string_lossy().to_string(),
        icon_data_url: icon_data_url(&icon_png_path.to_string_lossy()),
        desktop_shortcut_path: None,
        installed: false,
        task_name,
        created_at_unix_ms: now_unix_ms(),
    };

    programs.push(entry.clone());
    storage::save_programs(&paths, &programs)?;
    Ok(entry)
}

#[tauri::command]
fn install_programs(app: tauri::AppHandle, ids: Vec<String>) -> Result<usize, String> {
    let paths = storage::resolve_paths(&app)?;
    storage::ensure_dirs(&paths)?;

    let desktop = app
        .path()
        .desktop_dir()
        .map_err(|e| format!("resolve desktop dir failed: {e}"))?;

    let pending = elevate::PendingInstall {
        programs_json: paths.programs_json.to_string_lossy().to_string(),
        desktop_dir: desktop.to_string_lossy().to_string(),
        ids: ids.clone(),
    };
    let pending_path = paths.data_dir.join("pending_install.json");
    let text = serde_json::to_string_pretty(&pending)
        .map_err(|e| format!("serialize pending install failed: {e}"))?;
    std::fs::write(&pending_path, text).map_err(|e| {
        format!(
            "write pending install failed ({}): {e}",
            pending_path.display()
        )
    })?;

    // Prefer running the persistent installer scheduled task (no UAC). If it doesn't exist or is broken
    // (e.g., still points to an older exe path), fall back to a one-shot elevated process which will also
    // recreate the installer task.
    let ran_task_ok = tasks::run_task(elevate::INSTALLER_TASK_NAME).is_ok();

    // Wait briefly for the installer to finish (it deletes the pending file on success).
    for _ in 0..50 {
        if !pending_path.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // If the task "started" but didn't process the pending file (common when its action exe is missing),
    // fall back to an explicit elevation path.
    if pending_path.exists() {
        // If the task did exist, this will likely trigger UAC once and repair the scheduled task to the
        // current executable path.
        elevate::run_elevated_install(&pending_path)?;
        for _ in 0..50 {
            if !pending_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    } else if !ran_task_ok {
        // Task didn't run and pending is already gone (unexpected), but keep behavior consistent.
    }

    let programs = storage::load_programs(&paths)?;
    let count = programs
        .iter()
        .filter(|p| ids.iter().any(|id| id == &p.id) && p.installed)
        .count();
    Ok(count)
}

#[tauri::command]
fn run_program(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let paths = storage::resolve_paths(&app)?;
    let programs = storage::load_programs(&paths)?;
    let p = programs
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "program not found".to_string())?;
    if !p.installed {
        return Err("not installed yet".to_string());
    }
    tasks::run_task(&p.task_name)
}

#[tauri::command]
fn open_program_location(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let paths = storage::resolve_paths(&app)?;
    let programs = storage::load_programs(&paths)?;
    let p = programs
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "program not found".to_string())?;

    let target = std::path::PathBuf::from(p.target_path.trim_matches('"'));

    // Prefer opening the target exe's parent folder (what users typically want).
    // Fall back to the desktop shortcut folder if the exe is missing.
    let folder = if target.exists() {
        target
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| "target has no parent directory".to_string())?
    } else if let Some(shortcut) = &p.desktop_shortcut_path {
        let shortcut_path = std::path::PathBuf::from(shortcut.trim_matches('"'));
        shortcut_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| "shortcut has no parent directory".to_string())?
    } else {
        return Err("path does not exist".to_string());
    };

    // Use spawn() to avoid propagating Explorer's non-zero exit codes (it may still open successfully).
    std::process::Command::new("explorer.exe")
        .arg(folder)
        .spawn()
        .map_err(|e| format!("launch explorer failed: {e}"))?;

    Ok(())
}

#[tauri::command]
fn remove_program(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let paths = storage::resolve_paths(&app)?;
    storage::ensure_dirs(&paths)?;

    let mut programs = storage::load_programs(&paths)?;
    let p = programs
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| "program not found".to_string())?;

    // Drafts can be removed without elevation.
    if !p.installed {
        programs.retain(|x| x.id != id);
        let _ = std::fs::remove_file(&p.icon_path);
        storage::save_programs(&paths, &programs)?;
        return Ok(());
    }

    // Installed entries need elevated rights to reliably delete the scheduled task.
    let pending = elevate::PendingCleanup {
        programs_json: paths.programs_json.to_string_lossy().to_string(),
        ids: vec![id.clone()],
    };
    let pending_path = paths.data_dir.join("pending_cleanup.json");
    let text = serde_json::to_string_pretty(&pending)
        .map_err(|e| format!("serialize pending cleanup failed: {e}"))?;
    std::fs::write(&pending_path, text).map_err(|e| {
        format!(
            "write pending cleanup failed ({}): {e}",
            pending_path.display()
        )
    })?;

    let _ran_task_ok = tasks::run_task(elevate::CLEANER_TASK_NAME).is_ok();

    // Wait briefly for cleanup to finish (it deletes the pending file on success).
    for _ in 0..50 {
        if !pending_path.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Same "task exists but broken" fallback as install.
    if pending_path.exists() {
        elevate::run_elevated_cleanup(&pending_path)?;
        for _ in 0..50 {
            if !pending_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    Ok(())
}

#[tauri::command]
fn get_autostart_enabled() -> Result<bool, String> {
    autostart::is_enabled(AUTOSTART_VALUE_NAME)
}

#[tauri::command]
fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    // In `tauri dev`, the binary loads the UI from `devUrl` (localhost). If we register that debug
    // executable for autostart, Windows will launch it on boot when no dev server is running,
    // causing a "localhost refused to connect" blank window.
    if enabled && cfg!(debug_assertions) {
        return Err("Autostart is disabled for dev/debug builds. Please enable it from the installed release build (tauri build), or disable autostart and re-enable after installation.".to_string());
    }
    let exe = std::env::current_exe().map_err(|e| format!("resolve current exe failed: {e}"))?;
    let exe_str = exe.to_string_lossy();
    // Always quote paths since they may contain spaces.
    let cmd = format!("\"{}\"", exe_str);
    autostart::set_enabled(AUTOSTART_VALUE_NAME, enabled, &cmd)
}

#[tauri::command]
fn set_window_always_on_top(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.set_always_on_top(enabled)
        .map_err(|e| format!("set always on top failed: {e}"))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_programs,
            reorder_programs,
            add_program_draft_from_path,
            add_program_from_path,
            install_programs,
            run_program,
            open_program_location,
            remove_program,
            get_autostart_enabled,
            set_autostart_enabled,
            set_window_always_on_top
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
