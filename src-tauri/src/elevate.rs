use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::Threading::WaitForSingleObject,
        UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        },
    },
};

use crate::{storage, tasks, win_shortcut};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInstall {
    pub programs_json: String,
    pub desktop_dir: String,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCleanup {
    pub programs_json: String,
    pub ids: Vec<String>,
}

pub const INSTALLER_TASK_NAME: &str = "elevated_SkipUAC_Installer";
pub const CLEANER_TASK_NAME: &str = "elevated_SkipUAC_Cleaner";

fn to_wide_null(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

fn wait_process(handle: windows::Win32::Foundation::HANDLE) {
    // Best-effort wait; even if we time out, the tasks/shortcuts might still be created.
    let _ = unsafe { WaitForSingleObject(handle, 30_000) };
    let _ = unsafe { CloseHandle(handle) };
}

pub fn run_elevated_install(pending_path: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let params = format!("--install-pending \"{}\"", pending_path.display());

    let verb_w = to_wide_null("runas");
    let file_w = to_wide_null(&exe.to_string_lossy());
    let params_w = to_wide_null(&params);

    let mut info = SHELLEXECUTEINFOW::default();
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = PCWSTR(verb_w.as_ptr());
    info.lpFile = PCWSTR(file_w.as_ptr());
    info.lpParameters = PCWSTR(params_w.as_ptr());
    info.nShow = 0; // SW_HIDE

    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| format!("ShellExecuteExW failed: {e}"))?;

    if !info.hProcess.is_invalid() {
        wait_process(info.hProcess);
    }
    Ok(())
}

pub fn run_elevated_cleanup(pending_path: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let params = format!("--cleanup-pending \"{}\"", pending_path.display());

    let verb_w = to_wide_null("runas");
    let file_w = to_wide_null(&exe.to_string_lossy());
    let params_w = to_wide_null(&params);

    let mut info = SHELLEXECUTEINFOW::default();
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = PCWSTR(verb_w.as_ptr());
    info.lpFile = PCWSTR(file_w.as_ptr());
    info.lpParameters = PCWSTR(params_w.as_ptr());
    info.nShow = 0; // SW_HIDE

    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| format!("ShellExecuteExW failed: {e}"))?;

    if !info.hProcess.is_invalid() {
        wait_process(info.hProcess);
    }
    Ok(())
}

pub fn perform_pending_install(pending_path: &Path) -> Result<usize, String> {
    // Ensure a persistent installer task exists so future installs can run without UAC.
    // Also create a persistent cleaner task for removals.
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let args = format!("--install-pending \"{}\"", pending_path.display());
    let _ = tasks::create_task(INSTALLER_TASK_NAME, &exe.to_string_lossy(), Some(&args));
    let cleanup_path = pending_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("pending_cleanup.json");
    let cleanup_args = format!("--cleanup-pending \"{}\"", cleanup_path.display());
    let _ = tasks::create_task(CLEANER_TASK_NAME, &exe.to_string_lossy(), Some(&cleanup_args));

    let text = std::fs::read_to_string(pending_path)
        .map_err(|e| format!("read pending install failed ({}): {e}", pending_path.display()))?;
    let pending: PendingInstall =
        serde_json::from_str(&text).map_err(|e| format!("parse pending install failed: {e}"))?;

    let programs_json = PathBuf::from(&pending.programs_json);
    let desktop_dir = PathBuf::from(&pending.desktop_dir);

    let paths = storage::paths_from_programs_json(programs_json);
    storage::ensure_dirs(&paths)?;

    let mut programs = storage::load_programs(&paths)?;
    let mut installed_count = 0usize;
    for id in &pending.ids {
        let Some(p) = programs.iter_mut().find(|p| &p.id == id) else {
            continue;
        };
        if p.installed {
            continue;
        }
        let args_str = p.arguments.as_deref();
        tasks::create_task(&p.task_name, &p.target_path, args_str)?;
        let shortcut = win_shortcut::create_shortcut_in_dir(&desktop_dir, &p.name, &p.task_name, &p.target_path)?;
        p.desktop_shortcut_path = Some(shortcut);
        p.installed = true;
        installed_count += 1;
    }
    storage::save_programs(&paths, &programs)?;

    // Leave the pending file for debugging, but it can be safely removed by caller.
    let _ = std::fs::remove_file(pending_path);

    // Small delay to let Explorer refresh desktop icons in some cases.
    std::thread::sleep(Duration::from_millis(150));
    Ok(installed_count)
}

pub fn perform_pending_cleanup(pending_path: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(pending_path)
        .map_err(|e| format!("read pending cleanup failed ({}): {e}", pending_path.display()))?;
    let pending: PendingCleanup =
        serde_json::from_str(&text).map_err(|e| format!("parse pending cleanup failed: {e}"))?;

    let programs_json = PathBuf::from(&pending.programs_json);
    let paths = storage::paths_from_programs_json(programs_json);
    storage::ensure_dirs(&paths)?;

    let mut programs = storage::load_programs(&paths)?;
    let mut removed_count = 0usize;

    for id in &pending.ids {
        let Some(idx) = programs.iter().position(|p| &p.id == id) else {
            continue;
        };
        let p = programs.remove(idx);
        let _ = tasks::delete_task(&p.task_name);
        let _ = std::fs::remove_file(&p.icon_path);
        if let Some(sp) = &p.desktop_shortcut_path {
            let _ = std::fs::remove_file(sp);
        }
        removed_count += 1;
    }

    storage::save_programs(&paths, &programs)?;
    let _ = std::fs::remove_file(pending_path);
    Ok(removed_count)
}
