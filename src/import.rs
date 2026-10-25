//! Local files are exposed to Bevy through an in-memory asset source, so GLB
//! loading keeps Bevy's materials, textures, node names, and transforms intact.
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    gltf::{Gltf, GltfLoaderSettings},
    prelude::*,
};
use formcraft::validate_glb;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Resource)]
pub struct ImportFiles(pub bevy::asset::io::memory::Dir);
#[derive(Resource, Default, Clone)]
pub struct ImportQueue(pub Arc<Mutex<Vec<(String, Vec<u8>)>>>);
#[derive(Resource, Default)]
pub struct Imports {
    pub models: Vec<ImportedModel>,
    pub next_id: u64,
    pub error: Option<String>,
}
pub struct ImportedModel {
    pub name: String,
    pub handle: Handle<Gltf>,
    pub root: Option<Entity>,
    pub error: Option<String>,
    pub path: String,
}
#[derive(Component)]
pub struct NeedsGrounding;

pub fn pick_files(queue: ImportQueue) {
    let future = async move {
        if let Some(files) = rfd::AsyncFileDialog::new()
            .add_filter("Binary glTF", &["glb"])
            .pick_files()
            .await
        {
            for file in files {
                let name = file.file_name();
                let bytes = file.read().await;
                queue.0.lock().unwrap().push((name, bytes));
            }
        }
    };
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(future);
    #[cfg(not(target_arch = "wasm32"))]
    bevy::tasks::IoTaskPool::get().spawn(future).detach();
}

pub fn receive_files(
    queue: Res<ImportQueue>,
    files: Res<ImportFiles>,
    server: Res<AssetServer>,
    mut imports: ResMut<Imports>,
) {
    let incoming = std::mem::take(&mut *queue.0.lock().unwrap());
    for (name, bytes) in incoming {
        if let Err(error) = validate_glb(&bytes) {
            imports.error = Some(format!("{name}: {error}"));
            continue;
        }
        let path = format!("model-{}.glb", imports.next_id);
        imports.next_id += 1;
        files.0.insert_asset(Path::new(&path), bytes);
        let handle = server
            .load_builder()
            .with_settings(|settings: &mut GltfLoaderSettings| {
                settings.load_cameras = false;
                settings.load_lights = false;
            })
            .load(format!("import://{path}"));
        imports.models.push(ImportedModel {
            name,
            handle,
            root: None,
            error: None,
            path,
        });
        imports.error = None;
    }
}

pub fn finish_imports(
    mut commands: Commands,
    mut imports: ResMut<Imports>,
    gltfs: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
) {
    for model in &mut imports.models {
        if model.root.is_some() || model.error.is_some() {
            continue;
        }
        if let LoadState::Failed(error) = server.load_state(model.handle.id()) {
            model.error = Some(error.to_string());
            continue;
        }
        match server.recursive_dependency_load_state(model.handle.id()) {
            RecursiveDependencyLoadState::Failed(error) => {
                model.error = Some(error.to_string());
                continue;
            }
            RecursiveDependencyLoadState::Loaded => {}
            _ => continue,
        }
        if let Some(gltf) = gltfs.get(&model.handle) {
            if let Some(scene) = gltf
                .default_scene
                .clone()
                .or_else(|| gltf.scenes.first().cloned())
            {
                model.root = Some(
                    commands
                        .spawn((
                            Name::new(model.name.clone()),
                            WorldAssetRoot(scene),
                            Transform::default(),
                            NeedsGrounding,
                        ))
                        .id(),
                );
            } else {
                model.error = Some("No renderable scene found in this GLB.".into());
            }
        }
    }
}
