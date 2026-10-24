use serde_json::json;

use super::DescribeParams;
use crate::Session;
use crate::schema::input_schema_for;
use crate::schema::tests::{assert_keys, assert_strict_keeps_todays_errors};

#[test]
fn schemas_and_the_key_check_agree() {
    assert_keys(input_schema_for::<DescribeParams>(), &["command"]);
}

#[test]
fn strict_keeps_todays_errors() {
    assert_strict_keeps_todays_errors(&[
        ("engine.describe", json!({"command": 5})),
        ("engine.describe", json!({"command": null})),
        ("engine.describe", json!({"command": "solid.extrud"})),
    ]);
}

#[test]
fn describe_returns_both_schemas() {
    let mut s = Session::default();
    let d = s.execute("engine.describe", &json!({"command": "solid.box"})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(d["id"], "solid.box");
    assert!(d["input"]["properties"].get("length").is_some());
    assert!(d["output"]["properties"].get("created_bodies").is_some());
    // Its own output schema lists the warnings warn mode can add.
    let own = s.execute("engine.describe", &json!({"command": "engine.describe"})).unwrap_or_else(|e| panic!("{e}"));
    assert!(own["output"]["properties"].get("param_warnings").is_some(), "{own}");
    let d = s.execute("engine.describe", &json!({"command": "solid.revolve"})).unwrap_or_else(|e| panic!("{e}"));
    assert!(d["input"].is_null() && d["output"].is_null(), "unconverted commands have no schemas yet");
    let e = s.execute("engine.describe", &json!({"command": "solid.extrud"})).err().map(|e| e.to_string());
    assert!(e.as_deref().is_some_and(|e| e.contains("did you mean `solid.extrude`")), "{e:?}");
}
