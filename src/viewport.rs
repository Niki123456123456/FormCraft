use crate::{import::NeedsGrounding, ui::Workspace};
use bevy::{
    camera::primitives::Aabb,
    camera_controller::pan_orbit_camera::prelude::*,
    input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};

#[derive(Component)]
pub struct ModelCamera;

// Input ownership is decided when a drag starts; dragging across a panel does
// not interrupt it, and a drag started in a panel never moves the camera.
pub fn camera_input(
    state: Res<Workspace>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut camera: Query<&mut PanOrbitCamera, With<ModelCamera>>,
    mut dragging: Local<Option<MouseButton>>,
    mut zoom_idle: Local<f32>,
    time: Res<Time>,
) {
    let Ok(mut camera) = camera.single_mut() else {
        return;
    };
    let zoom: f32 = wheel
        .read()
        .map(|e| {
            e.y * if e.unit == MouseScrollUnit::Line {
                100.0
            } else {
                1.0
            }
        })
        .sum();
    if let Some(button) = *dragging {
        if !buttons.pressed(button) {
            camera.end_move();
            *dragging = None;
        }
    }
    if state.pointer_in_viewport && dragging.is_none() {
        let pan = buttons.just_pressed(MouseButton::Middle)
            || (buttons.just_pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft));
        let orbit = buttons.just_pressed(MouseButton::Right)
            || (buttons.just_pressed(MouseButton::Left) && !keys.pressed(KeyCode::ShiftLeft));
        if pan {
            camera.end_move();
            camera.start_pan(None);
            *dragging = Some(if buttons.just_pressed(MouseButton::Middle) {
                MouseButton::Middle
            } else {
                MouseButton::Left
            });
        } else if orbit {
            camera.end_move();
            camera.start_orbit(None);
            *dragging = Some(if buttons.just_pressed(MouseButton::Right) {
                MouseButton::Right
            } else {
                MouseButton::Left
            });
        }
    }
    if dragging.is_some() {
        camera.send_screenspace_input(motion.delta);
    }
    if state.pointer_in_viewport && zoom != 0.0 {
        *zoom_idle = 0.2;
        if dragging.is_none() {
            camera.start_zoom(None);
        }
        camera.send_zoom_input(zoom);
    }
    if zoom == 0.0 {
        *zoom_idle = (*zoom_idle - time.delta_secs()).max(0.0);
    }
    if dragging.is_none() && *zoom_idle == 0.0 {
        camera.end_move();
    }
}

pub fn draw_grid(state: Res<Workspace>, mut gizmos: Gizmos) {
    if !state.show_grid {
        return;
    }
    let extent = 50;
    for i in -extent..=extent {
        let n = i as f32;
        let color = if i % 5 == 0 {
            Color::srgb(0.18, 0.21, 0.25)
        } else {
            Color::srgb(0.125, 0.145, 0.177)
        };
        if i != 0 {
            gizmos.line(
                Vec3::new(n, 0.0, -extent as f32),
                Vec3::new(n, 0.0, extent as f32),
                color,
            );
            gizmos.line(
                Vec3::new(-extent as f32, 0.0, n),
                Vec3::new(extent as f32, 0.0, n),
                color,
            );
        }
    }
    gizmos.line(
        Vec3::NEG_X * 50.0,
        Vec3::X * 50.0,
        Color::srgb(0.64, 0.31, 0.34),
    );
    gizmos.line(
        Vec3::NEG_Z * 50.0,
        Vec3::Z * 50.0,
        Color::srgb(0.31, 0.46, 0.71),
    );
    gizmos.line(Vec3::ZERO, Vec3::Y * 1.5, Color::srgb(0.41, 0.68, 0.51));
}

/// World-space bounds include all transformed descendants, not just root meshes.
pub fn bounds(
    root: Entity,
    children: &Query<&Children>,
    meshes: &Query<(&Aabb, &GlobalTransform), With<Mesh3d>>,
) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
        if let Ok((aabb, transform)) = meshes.get(entity) {
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    for z in [-1.0, 1.0] {
                        let p = transform.transform_point(
                            Vec3::from(aabb.center)
                                + Vec3::from(aabb.half_extents) * Vec3::new(x, y, z),
                        );
                        min = min.min(p);
                        max = max.max(p);
                    }
                }
            }
        }
    }
    if min.is_finite() && max.is_finite() {
        Some((min, max))
    } else {
        None
    }
}

pub fn frame_bounds(
    min: Vec3,
    max: Vec3,
    transform: &mut Transform,
    projection: &mut Projection,
    controller: &mut PanOrbitCamera,
    aspect: f32,
) {
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(0.1);
    let distance = radius * 3.0 / aspect.min(1.0).max(0.1);
    controller.end_move();
    controller.current_motion = motion::CurrentMotion::Stationary;
    transform.translation = center - *transform.forward() * distance;
    controller.last_anchor_depth = -distance as f64;
    if let Projection::Orthographic(ortho) = projection {
        ortho.scale = radius * 2.8 / aspect.min(1.0).max(0.1);
    }
}

pub fn settle_imports(
    mut commands: Commands,
    mut roots: Query<(Entity, &mut Transform), (With<NeedsGrounding>, Without<ModelCamera>)>,
    children: Query<&Children>,
    meshes: Query<(&Aabb, &GlobalTransform), With<Mesh3d>>,
    mut camera: Query<
        (&mut Transform, &mut Projection, &mut PanOrbitCamera),
        (With<ModelCamera>, Without<NeedsGrounding>),
    >,
    state: Res<Workspace>,
) {
    for (entity, mut root_transform) in &mut roots {
        if let Some((min, max)) = bounds(entity, &children, &meshes) {
            // Center on XZ and rest the lowest point on the construction grid.
            let offset = Vec3::new(-(min.x + max.x) * 0.5, -min.y, -(min.z + max.z) * 0.5);
            root_transform.translation += offset;
            commands.entity(entity).remove::<NeedsGrounding>();
            if let Ok((mut transform, mut projection, mut controller)) = camera.single_mut() {
                frame_bounds(
                    min + offset,
                    max + offset,
                    &mut transform,
                    &mut projection,
                    &mut controller,
                    state.aspect(),
                );
            }
        }
    }
}
