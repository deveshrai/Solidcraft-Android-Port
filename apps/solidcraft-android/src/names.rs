//! Names for documents' local copies, and the MIME types documents are saved under.

/// A document's name as a file name in our folder: no folders, no hidden or empty names.
pub fn file_name(name: &str) -> String {
    let name: String = name.chars().map(|c| if c == '/' || c == '\\' || c == '\0' { '_' } else { c }).collect();
    let name = name.trim().trim_start_matches('.').to_string();
    if name.is_empty() { "document".into() } else { name }
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
    fn mime_types_follow_the_extension() {
        assert_eq!(mime("a.STL"), "model/stl");
        assert_eq!(mime("a.stp"), "model/step");
        assert_eq!(mime("design.solidcraft"), "application/octet-stream");
        assert_eq!(mime("noext"), "application/octet-stream");
    }
}
