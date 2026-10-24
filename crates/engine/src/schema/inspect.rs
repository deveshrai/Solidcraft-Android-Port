//! Contracts and result types for the commands in `cmd/inspect.rs`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Which command to describe.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct DescribeParams {
    /// The command's id, such as `solid.extrude`.
    #[serde(deserialize_with = "command_id")]
    pub command: String,
}

/// A command id, with the `engine.describe` handler's message for anything else.
fn command_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    match Value::deserialize(d)? {
        Value::String(s) => Ok(s),
        _ => Err(serde::de::Error::custom("must be a command id")),
    }
}

/// What `engine.describe` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DescribeResult {
    pub id: String,
    pub label: String,
    /// The command's parameter documentation, as prose.
    pub params: String,
    /// The parameters' JSON Schema, or `null` when the command has no contract yet.
    pub input: Option<Value>,
    /// The successful result's JSON Schema, or `null` when not described yet.
    pub output: Option<Value>,
    /// In the warn parameter-check mode: what the call's parameters got wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
