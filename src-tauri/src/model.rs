use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramEntry {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub target_path: String,
    pub arguments: Option<String>,
    pub icon_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_data_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_shortcut_path: Option<String>,
    #[serde(default)]
    pub installed: bool,
    pub task_name: String,
    pub created_at_unix_ms: i64,
}
