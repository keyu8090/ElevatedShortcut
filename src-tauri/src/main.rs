// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    // One-shot CLI mode used by the generated desktop shortcuts:
    // `SkipUAC.exe --run-task <task_name>`
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--run-task" {
            if let Some(task_name) = args.next() {
                let _ = SkipUAC_lib::run_task_cli(&task_name);
            }
            return;
        }
        if let Some(rest) = arg.strip_prefix("--run-task=") {
            if !rest.trim().is_empty() {
                let _ = SkipUAC_lib::run_task_cli(rest.trim());
            }
            return;
        }
        if arg == "--install-pending" {
            if let Some(pending_path) = args.next() {
                let _ = SkipUAC_lib::install_pending_cli(&pending_path);
            }
            return;
        }
        if let Some(rest) = arg.strip_prefix("--install-pending=") {
            if !rest.trim().is_empty() {
                let _ = SkipUAC_lib::install_pending_cli(rest.trim());
            }
            return;
        }
        if arg == "--cleanup-pending" {
            if let Some(pending_path) = args.next() {
                let _ = SkipUAC_lib::cleanup_pending_cli(&pending_path);
            }
            return;
        }
        if let Some(rest) = arg.strip_prefix("--cleanup-pending=") {
            if !rest.trim().is_empty() {
                let _ = SkipUAC_lib::cleanup_pending_cli(rest.trim());
            }
            return;
        }
    }
    SkipUAC_lib::run()
}
