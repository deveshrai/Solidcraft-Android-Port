//! A helical thread's hand is saved as upstream's `handedness` (`"left"`), which every Thread
//! has, instead of the port's own `left_handed: true`. Designs saved by port releases before
//! 0.24.0a are rewritten as they open: a Thread's `left_handed: true` becomes
//! `handedness: "left"`, and `left_handed` is dropped. A release without this change would read
//! a left-handed helical thread as right-handed, so it must ask for an update instead.
use serde_json::Value;

pub fn step(v: &mut Value) -> crate::Result<()> {
    let Some(Value::Array(features)) = v.get_mut("features") else {
        return Ok(());
    };
    for feature in features {
        let Some(object) = feature.as_object_mut() else { continue };
        if object.get("type").and_then(Value::as_str) != Some("thread") {
            continue;
        }
        if let Some(left) = object.remove("left_handed")
            && left.as_bool() == Some(true)
        {
            object.insert("handedness".into(), Value::String("left".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn a_left_handed_helical_thread_keeps_its_hand() {
        let mut v = json!({"features": [
            {"type": "thread", "helical": true, "left_handed": true},
            {"type": "thread", "helical": true, "left_handed": false},
            {"type": "thread", "modeled": true},
            {"type": "extrude", "left_handed": true}
        ]});
        super::step(&mut v).unwrap();
        assert_eq!(
            v["features"],
            json!([
                {"type": "thread", "helical": true, "handedness": "left"},
                {"type": "thread", "helical": true},
                {"type": "thread", "modeled": true},
                {"type": "extrude", "left_handed": true}
            ])
        );
        let mut empty = json!({"name": "Untitled"});
        super::step(&mut empty).unwrap();
        assert_eq!(empty, json!({"name": "Untitled"}));
    }
}
