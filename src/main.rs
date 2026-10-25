mod import;
mod ui;
mod viewport;

use bevy::{
    asset::{
        AssetMetaCheck,
        io::{
            AssetSourceBuilder,
            memory::{Dir, MemoryAssetReader},
        },
    },
    camera::{CameraOutputMode, visibility::RenderLayers},
    camera_controller::pan_orbit_camera::prelude::*,
    prelude::*,
};
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext};

fn main() {
    let files = Dir::default();
    let reader = files.clone();
    App::new()
        .register_asset_source(
            "import",
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: reader.clone(),
                })
            }),
        )
        .insert_resource(import::ImportFiles(files))
        .insert_resource(ClearColor(Color::srgb(0.094, 0.110, 0.137)))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 750.0,
            ..default()
        })
        .insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "FormCraft · 3D workspace".into(),
                        canvas: Some("#formcraft".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins((EguiPlugin::default(), DefaultPanOrbitCameraPlugins))
        .init_resource::<ui::Workspace>()
        .init_resource::<import::ImportQueue>()
        .init_resource::<import::Imports>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                import::receive_files,
                import::finish_imports,
                viewport::camera_input,
                viewport::draw_grid,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            viewport::settle_imports.after(TransformSystems::Propagate),
        )
        .add_systems(EguiPrimaryContextPass, ui::workspace_ui)
        .run();
}

fn setup(mut commands: Commands) {
    let translation = Vec3::new(9.0, 7.0, 11.0);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera {
            orthographic: projections::OrthographicSettings {
                scale_to_near_clip: 1000.0,
                ..default()
            },
            ..default()
        }
        .with_initial_anchor_depth(-translation.length() as f64),
        viewport::ModelCamera,
        Msaa::Sample4,
    ));
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        RenderLayers::none(),
        Camera {
            order: 1,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(bevy::render::render_resource::BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
