//! Names for documents' local copies, and the MIME types documents are saved under.

/// A document's name as a file name in our folder: no folders, no hidden or empty names.
pub fn file_name(name: &str) -> String {
    let name: String = name.chars().map(|c| if c == '/' || c == '\\' || c == '\0' { '_' } else { c }).collect();
    let name = name.trim().trim_start_matches('.').to_string();
    if name.is_empty() { "document".into() } else { name }
}

/// The name to save under: `chosen`, the document's name in the picker (what was typed there),
/// with the extension of `asked`, the name the app asked for, since the engine knows formats by
/// their extension. A provider's copy number after the extension ("part.stl (1)") moves before
/// it; a name with no extension, or another one, gets the asked one added.
pub fn save_name(chosen: &str, asked: &str) -> String {
    let chosen = chosen.trim();
    let Some((_, ext)) = asked.rsplit_once('.').filter(|(stem, ext)| !stem.is_empty() && !ext.is_empty()) else {
        return if chosen.is_empty() { asked.to_string() } else { chosen.to_string() };
    };
    if chosen.is_empty() {
        return asked.to_string();
    }
    let dot_ext = format!(".{}", ext.to_ascii_lowercase());
    let lower = chosen.to_ascii_lowercase();
    if lower.ends_with(&dot_ext) && chosen.len() > dot_ext.len() {
        return chosen.to_string();
    }
    // "part.stl (1)": the provider numbered a name that was taken.
    if let Some((before, copy)) = chosen.rsplit_once(' ')
        && copy.len() > 2
        && copy.starts_with('(')
        && copy.ends_with(')')
        && copy[1..copy.len() - 1].chars().all(|c| c.is_ascii_digit())
        && before.to_ascii_lowercase().ends_with(&dot_ext)
        && before.len() > dot_ext.len()
    {
        let stem = &before[..before.len() - dot_ext.len()];
        return format!("{stem} {copy}{}", &before[before.len() - dot_ext.len()..]);
    }
    format!("{chosen}.{ext}")
}

/// The MIME type a document picker files `name` under (what other apps see).
pub fn mime(name: &str) -> &'static str {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "stl" => "model/stl",
        "step" | "stp" => "model/step",
        "iges" | "igs" => "model/iges",
        "3mf" => "model/3mf",
        "obj" => "model/obj",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "dxf" => "image/vnd.dxf",
        "csv" => "text/csv",
        "urdf" | "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_stay_in_the_folder() {
        assert_eq!(file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(file_name(".hidden.step"), "hidden.step");
        assert_eq!(file_name("   "), "document");
        assert_eq!(file_name("Part 1.STEP"), "Part 1.STEP");
    }

    #[test]
    fn saves_keep_the_name_typed_in_the_picker() {
        assert_eq!(save_name("helical-m20.solidcraft", "Untitled.solidcraft"), "helical-m20.solidcraft");
        assert_eq!(save_name("gear", "Untitled.solidcraft"), "gear.solidcraft");
        assert_eq!(save_name("Part.STL", "Body1.stl"), "Part.STL");
        assert_eq!(save_name("part.stl (1)", "Body1.stl"), "part (1).stl");
        assert_eq!(save_name("notes.txt", "Body1.step"), "notes.txt.step");
        assert_eq!(save_name("  ", "Untitled.solidcraft"), "Untitled.solidcraft");
        assert_eq!(save_name(".stl", "Body1.stl"), ".stl.stl");
        assert_eq!(save_name("drawing", "noext"), "drawing");
    }

    #[test]
    fn mime_types_follow_the_extension() {
        assert_eq!(mime("a.STL"), "model/stl");
        assert_eq!(mime("a.stp"), "model/step");
        assert_eq!(mime("design.solidcraft"), "application/octet-stream");
        assert_eq!(mime("noext"), "application/octet-stream");
    }
}
