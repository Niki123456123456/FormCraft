//! Validation for self-contained binary glTF imports.
pub fn validate_glb(bytes: &[u8]) -> std::result::Result<(), String> {
    if bytes.len() < 12 || &bytes[..4] != b"glTF" {
        return Err("Choose a binary glTF (.glb) file.".into());
    }
    let glb = gltf::Gltf::from_slice(bytes).map_err(|e| format!("Invalid GLB: {e}"))?;
    if glb.document.scenes().next().is_none() {
        return Err("This GLB contains no scenes.".into());
    }
    let external_buffer = glb
        .document
        .buffers()
        .any(|b| matches!(b.source(), gltf::buffer::Source::Uri(uri) if !uri.starts_with("data:")));
    let external_image = glb.document.images().any(
        |i| matches!(i.source(), gltf::image::Source::Uri { uri, .. } if !uri.starts_with("data:")),
    );
    if external_buffer || external_image {
        return Err(
            "This GLB references external files. Export a GLB with embedded textures and buffers."
                .into(),
        );
    }
    if glb
        .document
        .extensions_required()
        .any(|e| e == "KHR_draco_mesh_compression" || e == "EXT_meshopt_compression")
    {
        return Err("Compressed meshes are not supported yet. Export an uncompressed GLB.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn glb(json: &str) -> Vec<u8> {
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let mut bytes = b"glTF".to_vec();
        bytes.extend(2_u32.to_le_bytes());
        bytes.extend(((20 + json.len()) as u32).to_le_bytes());
        bytes.extend((json.len() as u32).to_le_bytes());
        bytes.extend(b"JSON");
        bytes.extend(json);
        bytes
    }
    #[test]
    fn accepts_a_self_contained_scene() {
        assert!(
            validate_glb(&glb(
                r#"{"asset":{"version":"2.0"},"scenes":[{"nodes":[]}],"scene":0}"#
            ))
            .is_ok()
        );
    }
    #[test]
    fn accepts_transformed_hierarchy_fixture() {
        validate_glb(include_bytes!("../tests/fixtures/assembly.glb")).unwrap();
    }
    #[test]
    fn rejects_invalid_and_incomplete_imports() {
        assert!(validate_glb(b"not a model").is_err());
        assert!(validate_glb(&glb(r#"{"asset":{"version":"2.0"}}"#)).is_err());
        let external = glb(
            r#"{"asset":{"version":"2.0"},"scenes":[{"nodes":[]}],"buffers":[{"uri":"mesh.bin","byteLength":16}]}"#,
        );
        let error = validate_glb(&external).unwrap_err();
        assert!(error.contains("external"), "{error}");
    }
}
