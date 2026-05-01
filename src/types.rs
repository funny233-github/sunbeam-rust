use serde::{Deserialize, Serialize};

/// An extension manifest describing available commands and preferences.
///
/// Each extension script outputs this JSON when invoked with no arguments.
/// It tells sunbeam what commands the extension provides, which parameters
/// each command accepts, and what preferences the user can configure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Display name for the extension.
    pub title: String,
    /// One-line description shown in the extension list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Global preferences configurable once per extension (e.g. API keys).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Vec<Input>>,
    /// Every command the extension exposes.
    pub commands: Vec<CommandSpec>,
}

/// A single command inside an extension manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Machine-readable identifier used in payloads and CLI invocation.
    pub name: String,
    /// Human-readable label displayed in lists.
    pub title: String,
    /// When true, the command is omitted from the root list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// Parameters the command accepts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<Input>>,
    /// Determines how the command's output is rendered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<CommandMode>,
}

/// A typed input field used for both preferences and command parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    /// Must be one of `"string"`, `"boolean"`, or `"number"`.
    #[serde(rename = "type")]
    pub input_type: InputType,
    /// Machine-readable name, used as the key in params/preferences maps.
    pub name: String,
    /// Human-readable label displayed in the form UI.
    pub title: String,
    /// When true the user may leave this input empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    /// Fallback value when no user-supplied value is provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// The data type of an [`Input`] field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "number")]
    Number,
}

/// Determines how an extension command's output is displayed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandMode {
    /// Server-side search: query changes trigger a new extension invocation.
    #[serde(rename = "search")]
    Search,
    /// Client-side filter: items are filtered locally by fuzzy matching.
    #[serde(rename = "filter")]
    Filter,
    /// A full-page detail view with markdown or plain text.
    #[serde(rename = "detail")]
    Detail,
    /// Runs interactively in the parent terminal (no TUI).
    #[serde(rename = "tty")]
    Tty,
    /// Executes silently; no TUI shown, an optional notification may appear.
    #[serde(rename = "silent")]
    Silent,
}

/// The JSON envelope sent to an extension script as its sole argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// Name of the command to execute.
    pub command: String,
    /// Resolved preference values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
    /// Resolved parameter values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
    /// Working directory of the sunbeam process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Current search query (only set in `search` mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#query: Option<String>,
}

/// A list page returned by an extension in `search` or `filter` mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ListItem>>,
    /// Placeholder text when the list is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_text: Option<String>,
    /// When true, a detail panel is shown beside the item list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_detail: Option<bool>,
    /// Polling interval for auto-refresh (0 = no auto-refresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_refresh_seconds: Option<i32>,
    /// Actions available when no item is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

/// A selectable row inside a [`List`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    /// Used for history tracking and programmatic selection. Falls back to title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Primary display text.
    pub title: String,
    /// Secondary text shown beside the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Detail content shown when `show_detail` is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ListItemDetail>,
    /// Tag-like badges displayed at the end of the row (e.g. `["Oneliner"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessories: Option<Vec<String>>,
    /// Actions available when this item is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

/// Detail content embedded in a list item (shown in the side panel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItemDetail {
    /// Rendered with glamour-style markdown; takes precedence over `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Rendered as plain, word-wrapped text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A full-page detail view returned by an extension in `detail` mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
    /// Rendered with glamour-style markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Rendered as plain text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A user-triggerable action attached to a page or list item.
///
/// The `action_type` field determines which sub-struct is populated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Display label in the action bar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Keyboard shortcut character (e.g. `"c"` for `Alt+C`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Discriminator that selects the active payload variant.
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

/// Discriminator for the [`Action`] struct.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// Execute another command inside the same extension.
    #[serde(rename = "run")]
    Run,
    /// Open a URL or file path in the system default application.
    #[serde(rename = "open")]
    Open,
    /// Copy text to the system clipboard.
    #[serde(rename = "copy")]
    Copy,
    /// Open a file in the user's configured editor.
    #[serde(rename = "edit")]
    Edit,
    /// Run an arbitrary shell command.
    #[serde(rename = "exec")]
    Exec,
    /// Quit sunbeam.
    #[serde(rename = "exit")]
    Exit,
    /// Re-fetch the current page, optionally with new params.
    #[serde(rename = "reload")]
    Reload,
    /// Open the preferences form for a specific extension.
    #[serde(rename = "config")]
    Config,
}

/// Opens a URL or local file path in the default application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Copies a string to the system clipboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Whether to quit sunbeam after copying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

/// Invokes another command in the same or a different extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAction {
    /// Which extension to run. `None` means the current extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    /// Name of the command to execute.
    pub command: String,
    /// Arguments passed to the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
    /// Whether to re-fetch the current page after the command finishes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload: Option<bool>,
    /// Whether to quit sunbeam after the command finishes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

/// Runs a shell command, optionally interactively.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecAction {
    /// The shell command string.
    pub command: String,
    /// When true, the command takes over the terminal (TTY mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<bool>,
    /// Working directory for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// Whether to quit sunbeam after execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

/// Opens a file in the user's preferred editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditAction {
    /// Absolute or relative path to the file.
    pub path: String,
    /// Whether to quit sunbeam after the editor closes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
    /// Whether to re-fetch the current page after the editor closes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload: Option<bool>,
}

/// Opens the preferences configuration form for a given extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAction {
    /// Alias of the extension to configure.
    pub extension: String,
}

/// Re-fetches the current page, optionally replacing parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadAction {
    /// New parameter values to merge into the existing payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
}
