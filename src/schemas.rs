use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use once_cell::sync::OnceCell;

const LIST_SCHEMA: &str = include_str!("../schemas/list.schema.json");
const DETAIL_SCHEMA: &str = include_str!("../schemas/detail.schema.json");
const MANIFEST_SCHEMA: &str = include_str!("../schemas/manifest.schema.json");
const CONFIG_SCHEMA: &str = include_str!("../schemas/config.schema.json");
const ACTION_SCHEMA: &str = include_str!("../schemas/action.schema.json");
const PARAMS_SCHEMA: &str = include_str!("../schemas/params.schema.json");

/// Registers sub-schemas referenced via `$ref` (action.schema.json, params.schema.json)
/// and compiles the main schema with proper resolution.
fn compile_schema(schema_str: &str) -> Result<JSONSchema> {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_str).context("invalid embedded schema")?;
    let action_value: serde_json::Value =
        serde_json::from_str(ACTION_SCHEMA).context("invalid embedded action schema")?;
    let params_value: serde_json::Value =
        serde_json::from_str(PARAMS_SCHEMA).context("invalid embedded params schema")?;

    JSONSchema::options()
        .with_document("json-schema:///action.schema.json".to_string(), action_value)
        .with_document("json-schema:///params.schema.json".to_string(), params_value)
        .compile(&schema_value)
        .map_err(|e| anyhow::anyhow!("failed to compile schema: {e}"))
}

fn instance() -> &'static Schemas {
    static INSTANCE: OnceCell<Schemas> = OnceCell::new();
    INSTANCE.get_or_init(|| Schemas {
        list: compile_schema(LIST_SCHEMA).expect("BUG: LIST_SCHEMA embedded schema is invalid"),
        detail: compile_schema(DETAIL_SCHEMA).expect("BUG: DETAIL_SCHEMA embedded schema is invalid"),
        manifest: compile_schema(MANIFEST_SCHEMA).expect("BUG: MANIFEST_SCHEMA embedded schema is invalid"),
        config: compile_schema(CONFIG_SCHEMA).expect("BUG: CONFIG_SCHEMA embedded schema is invalid"),
    })
}

struct Schemas {
    list: JSONSchema,
    detail: JSONSchema,
    manifest: JSONSchema,
    config: JSONSchema,
}

fn validate(schema: &JSONSchema, input: &[u8]) -> Result<()> {
    let value: serde_json::Value = serde_json::from_slice(input).context("failed to parse JSON")?;
    if let Err(errors) = schema.validate(&value) {
        let mut msgs: Vec<String> = Vec::new();
        for err in errors {
            msgs.push(format!("{}: {}", err.instance_path, err));
        }
        anyhow::bail!("validation failed: {}", msgs.join("; "));
    }
    Ok(())
}

/// Validates a JSON byte slice against the list schema.
///
/// # Errors
/// Returns an error if the JSON is malformed or does not conform to the schema.
pub fn validate_list(input: &[u8]) -> Result<()> {
    validate(&instance().list, input)
}

/// Validates a JSON byte slice against the detail schema.
///
/// # Errors
/// Returns an error if the JSON is malformed or does not conform to the schema.
pub fn validate_detail(input: &[u8]) -> Result<()> {
    validate(&instance().detail, input)
}

/// Validates a JSON byte slice against the manifest schema.
///
/// # Errors
/// Returns an error if the JSON is malformed or does not conform to the schema.
pub fn validate_manifest(input: &[u8]) -> Result<()> {
    validate(&instance().manifest, input)
}

/// Validates a JSON byte slice against the config schema.
///
/// # Errors
/// Returns an error if the JSON is malformed or does not conform to the schema.
pub fn validate_config(input: &[u8]) -> Result<()> {
    validate(&instance().config, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_list_valid() {
        let list_json = serde_json::json!({
            "items": [
                {
                    "title": "Test Item",
                    "subtitle": "A test",
                    "accessories": ["tag"],
                    "actions": [{"title": "Run", "type": "run", "command": "test", "run": {"command": "test"}}]
                }
            ]
        });
        let result = validate_list(&serde_json::to_vec(&list_json).unwrap());
        assert!(result.is_ok(), "valid list should pass: {:?}", result.err());
    }

    #[test]
    fn test_validate_list_invalid() {
        let bad_json = br#"{"items": [{"title": 123}]}"#;
        let result = validate_list(bad_json);
        assert!(result.is_err(), "invalid list should fail");
    }

    #[test]
    fn test_validate_detail_valid() {
        let detail_json = serde_json::json!({
            "markdown": "# Hello\nThis is detail content.\n",
            "actions": []
        });
        let result = validate_detail(&serde_json::to_vec(&detail_json).unwrap());
        assert!(result.is_ok(), "valid detail should pass: {:?}", result.err());
    }

    #[test]
    fn test_validate_detail_invalid() {
        let bad_json = br#"{"markdown": 42}"#;
        let result = validate_detail(bad_json);
        assert!(result.is_err(), "invalid detail should fail");
    }

    #[test]
    fn test_validate_manifest_valid() {
        let manifest_json = serde_json::json!({
            "title": "Test Extension",
            "commands": [
                {"name": "cmd1", "title": "Command 1", "mode": "filter"}
            ]
        });
        let result = validate_manifest(&serde_json::to_vec(&manifest_json).unwrap());
        assert!(result.is_ok(), "valid manifest should pass: {:?}", result.err());
    }

    #[test]
    fn test_validate_manifest_invalid() {
        let bad_json = br#"{"title": 42}"#;
        let result = validate_manifest(bad_json);
        assert!(result.is_err(), "invalid manifest should fail");
    }

    #[test]
    fn test_validate_config_valid() {
        let cfg_json = serde_json::json!({
            "oneliners": [{"title": "Test", "command": "echo hello"}]
        });
        let result = validate_config(&serde_json::to_vec(&cfg_json).unwrap());
        assert!(result.is_ok(), "valid config should pass: {:?}", result.err());
    }
}
