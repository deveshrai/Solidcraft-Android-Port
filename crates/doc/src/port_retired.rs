//! SolidCraft Android port: design-format changes earlier port releases made and later took out.
//!
//! Port releases up to 0.40.0-rc.3b saved helical B-rep threads (a custom pitch, depth, flank
//! angle, clearance, offsets and start angle) as pending format changes, named in a design's
//! `format_pending`. The port dropped them for upstream's own modelled threads, so a build without
//! them would read those names as a newer build's and refuse the design. Instead they are known
//! here: their names leave `format_pending` (the design still gets every upstream change of its
//! number) and each helical thread becomes upstream's modelled thread. An ISO designation keeps
//! its size and pitch; a custom pitch gives way to the size the face takes. What the thread had
//! besides is kept, word for word, in the design's metadata under [`METADATA_KEY`], so nothing
//! saved is lost.
use serde_json::{Map, Value, json};

/// The pending changes earlier port releases wrote.
pub const RETIRED: [&str; 3] = ["helical-thread-handedness", "helical-thread-start-angle", "helical-threads"];

/// The design metadata key the retired threads' own fields are kept under.
pub const METADATA_KEY: &str = "io.github.deveshrai.solidcraft-android";

/// The helical thread's fields upstream's Thread doesn't have.
const HELICAL_FIELDS: [&str; 8] = ["helical", "pitch", "depth", "angle", "clearance", "start_offset", "end_offset", "start_angle"];

/// Whether `applied` names a retired change and nothing a newer build made: what [`take`] would
/// leave, for a check that reads no further (`says_newer`).
pub fn without_retired(applied: Option<Vec<String>>) -> Option<Vec<String>> {
    applied.map(|names| names.into_iter().filter(|n| !RETIRED.contains(&n.as_str())).collect())
}

/// Take the retired changes out of `applied` and rewrite what they saved in `v` as upstream's.
pub fn take(v: &mut Value, applied: Option<Vec<String>>) -> Option<Vec<String>> {
    let had = applied.as_ref().is_some_and(|names| names.iter().any(|n| RETIRED.contains(&n.as_str())));
    if had {
        rewrite_threads(v);
    }
    without_retired(applied)
}

fn rewrite_threads(v: &mut Value) {
    let mut kept = Vec::new();
    if let Some(Value::Array(features)) = v.get_mut("features") {
        for feature in features.iter_mut() {
            let Some(o) = feature.as_object_mut() else { continue };
            if o.get("type").and_then(Value::as_str) != Some("thread") {
                continue;
            }
            // The port's oldest hand, before upstream had one.
            if let Some(left) = o.remove("left_handed")
                && left.as_bool() == Some(true)
            {
                o.insert("hand".into(), json!("left"));
            }
            let helical = o.get("helical").and_then(Value::as_bool) == Some(true) || o.contains_key("pitch");
            if !helical {
                continue;
            }
            let mut fields = Map::new();
            for name in HELICAL_FIELDS {
                if let Some(value) = o.remove(name) {
                    fields.insert(name.into(), value);
                }
            }
            o.insert("modeled".into(), json!(true));
            let mut entry = Map::new();
            for name in ["id", "name"] {
                if let Some(value) = o.get(name) {
                    entry.insert(name.into(), value.clone());
                }
            }
            entry.insert("fields".into(), Value::Object(fields));
            kept.push(Value::Object(entry));
        }
    }
    if kept.is_empty() {
        return;
    }
    let Some(design) = v.as_object_mut() else { return };
    let metadata = design.entry("metadata").or_insert_with(|| json!({}));
    let Some(metadata) = metadata.as_object_mut() else { return };
    let ours = metadata.entry(METADATA_KEY).or_insert_with(|| json!({}));
    if let Some(ours) = ours.as_object_mut() {
        ours.insert("retired_helical_threads".into(), Value::Array(kept));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Option<Vec<String>> {
        Some(n.iter().map(|s| s.to_string()).collect())
    }

    /// A port design's helical threads become upstream's modelled threads, their own fields kept
    /// in the design's metadata; its other features and upstream's own pending changes stay.
    #[test]
    fn helical_threads_of_earlier_port_releases_become_modelled_threads() {
        let mut v = json!({"features": [
            {"id": 6, "name": "Thread1", "type": "thread", "designation": "M10", "modeled": true, "hand": "left", "helical": true, "start_angle": "90 deg"},
            {"id": 9, "name": "Thread2", "type": "thread", "pitch": "2.2 mm", "clearance": "0.15 mm", "start_offset": "-2.2 mm"},
            {"id": 11, "type": "thread", "designation": "M8"},
            {"id": 12, "type": "thread", "helical": true, "left_handed": true, "designation": "M20"},
            {"id": 13, "type": "extrude", "pitch": "keep"}
        ]});
        let left = take(&mut v, names(&["helical-thread-handedness", "upstream-change", "helical-threads"]));
        assert_eq!(left, names(&["upstream-change"]));
        let f = &v["features"];
        assert_eq!(f[0], json!({"id": 6, "name": "Thread1", "type": "thread", "designation": "M10", "modeled": true, "hand": "left"}));
        assert_eq!(f[1], json!({"id": 9, "name": "Thread2", "type": "thread", "modeled": true}));
        assert_eq!(f[2], json!({"id": 11, "type": "thread", "designation": "M8"}), "a cosmetic thread stays cosmetic");
        assert_eq!(f[3], json!({"id": 12, "type": "thread", "designation": "M20", "modeled": true, "hand": "left"}));
        assert_eq!(f[4], json!({"id": 13, "type": "extrude", "pitch": "keep"}));
        let kept = &v["metadata"][METADATA_KEY]["retired_helical_threads"];
        assert_eq!(kept[1], json!({"id": 9, "name": "Thread2", "fields": {"pitch": "2.2 mm", "clearance": "0.15 mm", "start_offset": "-2.2 mm"}}));
        assert_eq!(kept[0]["fields"], json!({"helical": true, "start_angle": "90 deg"}));
        assert_eq!(kept.as_array().map(Vec::len), Some(3));
    }

    /// A design no port release touched is left as it was.
    #[test]
    fn designs_without_retired_changes_are_untouched() {
        let before = json!({"features": [{"type": "thread", "pitch": "1 mm"}], "metadata": {"author": "A"}});
        for applied in [None, names(&[]), names(&["upstream-change"])] {
            let mut v = before.clone();
            assert_eq!(take(&mut v, applied.clone()), applied);
            assert_eq!(v, before);
        }
    }
}
