use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::Manager;

use crate::model::ProgramEntry;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub programs_json: PathBuf,
    pub icons_dir: PathBuf,
}

pub fn paths_from_programs_json(programs_json: PathBuf) -> AppPaths {
    let data_dir = programs_json
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let icons_dir = data_dir.join("icons");
    AppPaths {
        data_dir,
        programs_json,
        icons_dir,
    }
}

pub fn resolve_paths(app: &tauri::AppHandle) -> Result<AppPaths, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir failed: {e}"))?;
    let programs_json = data_dir.join("programs.json");
    let icons_dir = data_dir.join("icons");
    Ok(AppPaths {
        data_dir,
        programs_json,
        icons_dir,
    })
}

pub fn ensure_dirs(paths: &AppPaths) -> Result<(), String> {
    ensure_dir(&paths.data_dir)?;
    ensure_dir(&paths.icons_dir)?;
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("create dir failed ({}): {e}", path.display()))
}

pub fn load_programs(paths: &AppPaths) -> Result<Vec<ProgramEntry>, String> {
    if !paths.programs_json.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&paths.programs_json).map_err(|e| {
        format!(
            "read programs.json failed ({}): {e}",
            paths.programs_json.display()
        )
    })?;
    serde_json::from_str::<Vec<ProgramEntry>>(&text).map_err(|e| format!("parse programs.json failed: {e}"))
}

pub fn save_programs(paths: &AppPaths, programs: &[ProgramEntry]) -> Result<(), String> {
    let mut cleaned: Vec<ProgramEntry> = Vec::with_capacity(programs.len());
    for p in programs {
        let mut c = p.clone();
        c.icon_data_url = None;
        cleaned.push(c);
    }
    let text =
        serde_json::to_string_pretty(&cleaned).map_err(|e| format!("serialize programs failed: {e}"))?;
    fs::write(&paths.programs_json, text).map_err(|e| {
        format!(
            "write programs.json failed ({}): {e}",
            paths.programs_json.display()
        )
    })
}
