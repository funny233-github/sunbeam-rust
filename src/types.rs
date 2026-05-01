use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Vec<Input>>,
    pub commands: Vec<CommandSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<Input>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<CommandMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    #[serde(rename = "type")]
    pub input_type: InputType,
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "number")]
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandMode {
    #[serde(rename = "search")]
    Search,
    #[serde(rename = "filter")]
    Filter,
    #[serde(rename = "detail")]
    Detail,
    #[serde(rename = "tty")]
    Tty,
    #[serde(rename = "silent")]
    Silent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ListItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_detail: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_refresh_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ListItemDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItemDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(rename = "type")]
    pub action_type: ActionType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<OpenAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy: Option<CopyAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<RunAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<ExecAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<EditAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ConfigAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload: Option<ReloadAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    #[serde(rename = "run")]
    Run,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "copy")]
    Copy,
    #[serde(rename = "edit")]
    Edit,
    #[serde(rename = "exec")]
    Exec,
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "config")]
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecAction {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditAction {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAction {
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
}
