use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use windows::{
    core::{Interface, PCWSTR},
    Win32::Foundation::RPC_E_CHANGED_MODE,
    Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, IPersistFile,
    },
    Win32::UI::Shell::{IShellLinkW, ShellLink},
};

use crate::util::to_wide;
use crate::util::to_wide_str;

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn unique_shortcut_path(dir: &Path, base_name: &str) -> PathBuf {
    let clean = base_name
        .replace('\\', "_")
        .replace('/', "_")
        .replace(':', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "_")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_")
        .trim()
        .to_string();
    let base = if clean.is_empty() { "Elevated".to_string() } else { clean };

    let mut candidate = dir.join(format!("{base}.lnk"));
    if !candidate.exists() {
        return candidate;
    }
    for i in 2..1000 {
        candidate = dir.join(format!("{base} ({i}).lnk"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{base}-{}.lnk", now_unix_ms()))
}

fn create_shortcut_file(
    out_path: &Path,
    task_name: &str,
    icon_path: &str,
    description: &str,
) -> Result<(), String> {
    let launcher = std::env::current_exe()
        .map_err(|e| format!("resolve current exe failed: {e}"))?;
    let args = format!("--run-task \"{task_name}\"");

    let mut should_uninit = false;
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr == RPC_E_CHANGED_MODE {
            // COM already initialized with a different apartment model on this thread.
        } else {
            hr.ok().map_err(|e| format!("CoInitializeEx failed: {e}"))?;
            should_uninit = true;
        }
    }

    let result = (|| -> Result<(), String> {
        let shell_link: IShellLinkW = unsafe {
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance(ShellLink) failed: {e}"))?
        };

        let launcher_w = to_wide(&launcher);
        unsafe { shell_link.SetPath(PCWSTR(launcher_w.as_ptr())) }
            .map_err(|e| format!("IShellLinkW::SetPath failed: {e}"))?;

        let args_w = to_wide_str(&args);
        unsafe { shell_link.SetArguments(PCWSTR(args_w.as_ptr())) }
            .map_err(|e| format!("IShellLinkW::SetArguments failed: {e}"))?;

        let icon_w = to_wide(Path::new(icon_path));
        let _ = unsafe { shell_link.SetIconLocation(PCWSTR(icon_w.as_ptr()), 0) };

        let desc_w = to_wide_str(description);
        let _ = unsafe { shell_link.SetDescription(PCWSTR(desc_w.as_ptr())) };

        let persist: IPersistFile = shell_link
            .cast()
            .map_err(|e| format!("cast to IPersistFile failed: {e}"))?;
        let out_w = to_wide(out_path);
        unsafe { persist.Save(PCWSTR(out_w.as_ptr()), true) }
            .map_err(|e| format!("IPersistFile::Save failed: {e}"))?;
        Ok(())
    })();

    if should_uninit {
        unsafe { CoUninitialize() };
    }
    result
}

pub fn create_shortcut_in_dir(
    dir: &Path,
    display_name: &str,
    task_name: &str,
    icon_path: &str,
) -> Result<String, String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("create shortcut dir failed ({}): {e}", dir.display()))?;

    let name = format!("{display_name} (elevated)");
    let out_path = unique_shortcut_path(dir, &name);
    let desc = format!("Run {display_name} with highest privileges via Task Scheduler");
    create_shortcut_file(&out_path, task_name, icon_path, &desc)?;
    Ok(out_path.to_string_lossy().to_string())
}
