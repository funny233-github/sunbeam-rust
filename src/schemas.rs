use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use once_cell::sync::OnceCell;

const LIST_SCHEMA: &str = include_str!("../schemas/list.schema.json");
const DETAIL_SCHEMA: &str = include_str!("../schemas/detail.schema.json");
const MANIFEST_SCHEMA: &str = include_str!("../schemas/manifest.schema.json");
const CONFIG_SCHEMA: &str = include_str!("../schemas/config.schema.json");
#[allow(dead_code)]
const ACTION_SCHEMA: &str = include_str!("../schemas/action.schema.json");
#[allow(dead_code)]
const PARAMS_SCHEMA: &str = include_str!("../schemas/params.schema.json");

fn compile_schema(schema_str: &str) -> JSONSchema {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_str).expect("invalid embedded schema");
    JSONSchema::compile(&schema_value).expect("failed to compile schema")
}

fn instance() -> &'static Schemas {
    static INSTANCE: OnceCell<Schemas> = OnceCell::new();
    INSTANCE.get_or_init(|| Schemas {
        list: compile_schema(LIST_SCHEMA),
        detail: compile_schema(DETAIL_SCHEMA),
        manifest: compile_schema(MANIFEST_SCHEMA),
        config: compile_schema(CONFIG_SCHEMA),
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
