use crate::{
    import::{self, ImportFiles, ImportQueue, Imports},
    viewport::{self, ModelCamera},
};
use bevy::{
    camera::{Viewport, primitives::Aabb},
    camera_controller::pan_orbit_camera::{
        extensions::{dolly_zoom::DollyZoomTrigger, look_to::LookToTrigger},
        prelude::*,
    },
    math::DVec3,
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    EguiContexts,
    egui::{self, Color32, LayerId, Pos2, RichText, Stroke, Ui, UiBuilder},
};

const ACCENT: Color32 = Color32::from_rgb(167, 217, 189);
const MUTED: Color32 = Color32::from_rgb(143, 154, 172);

#[derive(Resource)]
pub struct Workspace {
    pub show_grid: bool,
    pub orthographic: bool,
    pub selected: Option<Entity>,
    pub pointer_in_viewport: bool,
    pub viewport: egui::Rect,
    pub help: bool,
    pub ready_frames: u8,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            show_grid: true,
            orthographic: false,
            selected: None,
            pointer_in_viewport: false,
            viewport: egui::Rect::NOTHING,
            help: false,
            ready_frames: 0,
        }
    }
}
impl Workspace {
    pub fn aspect(&self) -> f32 {
        self.viewport.width() / self.viewport.height().max(1.0)
    }
}

fn theme(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::from_rgb(28, 32, 40);
    style.visuals.window_fill = Color32::from_rgb(32, 37, 46);
    style.visuals.extreme_bg_color = Color32::from_rgb(23, 27, 34);
    style.visuals.faint_bg_color = Color32::from_rgb(36, 42, 51);
    style.visuals.selection.bg_fill = Color32::from_rgb(54, 80, 70);
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(42, 48, 59);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 41, 50);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(57, 67, 78);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(60, 85, 73);
    style.spacing.item_spacing = egui::vec2(10.0, 9.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_style_of(egui::Theme::Dark, style);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn workspace_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<Workspace>,
    mut imports: ResMut<Imports>,
    queue: Res<ImportQueue>,
    files: Res<ImportFiles>,
    mut commands: Commands,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<
        (
            Entity,
            &mut Camera,
            &mut Transform,
            &mut Projection,
            &mut PanOrbitCamera,
        ),
        With<ModelCamera>,
    >,
    names: Query<(Option<&Name>, Has<Mesh3d>)>,
    children: Query<&Children>,
    meshes: Query<(&Aabb, &GlobalTransform), With<Mesh3d>>,
    mut look: MessageWriter<LookToTrigger>,
    mut dolly: MessageWriter<DollyZoomTrigger>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    if state.ready_frames == 0 {
        theme(ctx);
    }
    state.ready_frames = state.ready_frames.saturating_add(1);
    #[cfg(target_arch = "wasm32")]
    if state.ready_frames == 3 {
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            if let Some(canvas) = document.get_element_by_id("formcraft") {
                let _ = canvas.set_attribute("data-ready", "true");
            }
        }
    }
    let Ok((camera_entity, mut camera, mut transform, mut projection, mut controller)) =
        cameras.single_mut()
    else {
        return Ok(());
    };
    let mut frame = false;
    let mut home = false;
    let mut toggle_projection = false;
    let mut remove = None;
    let mut facing = None;
    let mut ui = Ui::new(
        ctx.clone(),
        "workspace".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    egui::Panel::top("titlebar")
        .exact_size(60.0)
        .show(&mut ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new("F").size(29.0).strong().color(ACCENT));
                ui.label(RichText::new("FormCraft").size(23.0).strong());
                ui.add_space(14.0);
                ui.label(RichText::new("3D WORKSPACE").size(10.0).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("+  Import GLB").color(Color32::from_rgb(23, 39, 30)),
                            )
                            .fill(ACCENT),
                        )
                        .clicked()
                    {
                        import::pick_files(queue.clone());
                    }
                    if ui.button("?  Help").clicked() {
                        state.help = !state.help;
                    }
                });
            });
        });
    egui::Panel::bottom("status")
        .exact_size(30.0)
        .show(&mut ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("Ready").color(ACCENT).size(9.0));
                ui.label(
                    RichText::new(format!(
                        "{} imported model{}",
                        imports.models.len(),
                        if imports.models.len() == 1 { "" } else { "s" }
                    ))
                    .size(11.0),
                );
                ui.separator();
                ui.label(
                    RichText::new("Y up  ·  1 grid square = 1 unit")
                        .color(MUTED)
                        .size(11.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Local files · nothing uploaded")
                            .color(MUTED)
                            .size(11.0),
                    );
                });
            });
        });
    egui::Panel::left("hierarchy")
        .default_size(265.0)
        .min_size(210.0)
        .max_size(420.0)
        .resizable(true)
        .show(&mut ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("SCENE").size(11.0).strong().color(MUTED));
                ui.label(
                    RichText::new(format!("  {}", imports.models.len()))
                        .size(11.0)
                        .color(ACCENT),
                );
            });
            ui.add_space(9.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - 150.0).max(80.0))
                .show(ui, |ui| {
                    if imports.models.is_empty() {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("3D").size(30.0).color(MUTED));
                            ui.add_space(8.0);
                            ui.label(RichText::new("Your scene starts here").strong());
                            ui.label(
                                RichText::new(
                                    "Import a GLB to explore its\nobjects and hierarchy.",
                                )
                                .color(MUTED),
                            );
                            ui.add_space(10.0);
                            if ui.button("Import your first model").clicked() {
                                import::pick_files(queue.clone());
                            }
                        });
                    }
                    for (index, model) in imports.models.iter().enumerate() {
                        ui.push_id(index, |ui| {
                            if let Some(root) = model.root {
                                tree(ui, root, &names, &children, &mut state.selected, true, 0);
                            } else {
                                ui.horizontal(|ui| {
                                    if model.error.is_none() {
                                        ui.spinner();
                                    }
                                    ui.label(&model.name);
                                });
                            }
                            if let Some(error) = &model.error {
                                ui.colored_label(Color32::from_rgb(239, 154, 151), error);
                                if ui.small_button("Dismiss failed import").clicked() {
                                    remove = Some(index);
                                }
                            }
                            ui.add_space(3.0);
                        });
                    }
                });
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            if let Some(selected) = state.selected {
                let label = names
                    .get(selected)
                    .ok()
                    .and_then(|(n, _)| n)
                    .map(Name::as_str)
                    .unwrap_or("Object");
                ui.label(RichText::new("SELECTED OBJECT").size(10.0).color(MUTED));
                ui.label(RichText::new(label).strong());
                ui.horizontal(|ui| {
                    if ui.button("Frame selection").clicked() {
                        frame = true;
                    }
                    if ui
                        .button("Remove")
                        .on_hover_text("Remove the containing imported model")
                        .clicked()
                    {
                        remove = imports.models.iter().position(|m| {
                            m.root.is_some_and(|root| {
                                root == selected
                                    || children.iter_descendants(root).any(|e| e == selected)
                            })
                        });
                    }
                });
            } else {
                ui.label(
                    RichText::new(
                        "Select an object in the hierarchy\nto frame or remove its model.",
                    )
                    .color(MUTED)
                    .size(12.0),
                );
            }
        });
    egui::Panel::top("viewport_tools")
        .exact_size(48.0)
        .show(&mut ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(9.0);
                ui.label(RichText::new("Viewport").strong());
                ui.separator();
                if ui
                    .selectable_label(!state.orthographic, "Perspective")
                    .clicked()
                    && state.orthographic
                {
                    toggle_projection = true;
                }
                if ui
                    .selectable_label(state.orthographic, "Orthographic")
                    .clicked()
                    && !state.orthographic
                {
                    toggle_projection = true;
                }
                ui.separator();
                ui.toggle_value(&mut state.show_grid, "Grid");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Home").clicked() {
                        home = true;
                    }
                    if ui.button("Frame all").clicked() {
                        state.selected = None;
                        frame = true;
                    }
                });
            });
        });
    let rect = ui.available_rect_before_wrap();
    state.viewport = rect;
    let cube_rect = egui::Rect::from_min_size(
        egui::pos2((rect.right() - 178.0).max(rect.left()), rect.top() + 16.0),
        egui::vec2(164.0, 224.0),
    );
    // Cube uses its own foreground area; the rest of the viewport stays available to Bevy.
    egui::Area::new("viewcube".into())
        .fixed_pos(cube_rect.min)
        .show(ctx, |ui| {
            ui.set_min_width(164.0);
            facing = orientation_cube(ui, transform.rotation);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("ORIENTATION").size(9.0).color(MUTED));
                ui.horizontal(|ui| {
                    for (label, dir) in [
                        ("Top", Vec3::NEG_Y),
                        ("Front", Vec3::NEG_Z),
                        ("Right", Vec3::NEG_X),
                    ] {
                        if ui.small_button(label).clicked() {
                            facing = Some(dir);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    for (label, dir) in [("Bottom", Vec3::Y), ("Back", Vec3::Z), ("Left", Vec3::X)]
                    {
                        if ui.small_button(label).clicked() {
                            facing = Some(dir);
                        }
                    }
                });
            });
        });
    state.pointer_in_viewport = ctx
        .pointer_hover_pos()
        .is_some_and(|p| rect.contains(p) && !cube_rect.contains(p))
        && !state.help;
    let painter = ctx.layer_painter(LayerId::background());
    painter.text(
        rect.left_top() + egui::vec2(18.0, 20.0),
        egui::Align2::LEFT_TOP,
        if state.orthographic {
            "ORTHOGRAPHIC"
        } else {
            "PERSPECTIVE"
        },
        egui::FontId::proportional(10.0),
        MUTED,
    );
    painter.text(
        rect.left_bottom() + egui::vec2(18.0, -18.0),
        egui::Align2::LEFT_BOTTOM,
        "Drag to orbit   ·   Scroll to zoom   ·   Shift + drag to pan",
        egui::FontId::proportional(12.0),
        MUTED,
    );
    if imports.models.is_empty() {
        painter.text(
            rect.center() + egui::vec2(0.0, -36.0),
            egui::Align2::CENTER_CENTER,
            "A little space for your next big idea.",
            egui::FontId::proportional(20.0),
            Color32::from_rgb(174, 187, 202),
        );
        painter.text(
            rect.center() + egui::vec2(0.0, -7.0),
            egui::Align2::CENTER_CENTER,
            "Import a GLB model to get started",
            egui::FontId::proportional(13.0),
            MUTED,
        );
    }
    if let Some(error) = imports.error.clone() {
        egui::Window::new("Import failed")
            .collapsible(false)
            .show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(error);
                if ui.button("Close").clicked() {
                    imports.error = None;
                }
            });
    }
    if state.help {
        egui::Window::new("Welcome to FormCraft").collapsible(false).open(&mut state.help).show(ctx, |ui| {
            ui.label("A 3D inspection workspace built with Bevy and egui.");
            ui.separator();
            egui::Grid::new("help_controls").spacing([24.0, 12.0]).show(ui, |ui| {
                for (key, action) in [("Left or right drag", "Orbit the view"), ("Shift + drag / middle drag", "Pan the view"), ("Mouse wheel / trackpad scroll", "Zoom in and out"), ("Cube face / view labels", "Snap to a standard view"), ("Perspective / Orthographic", "Switch camera projection"), ("Frame all / Frame selection", "Fit models into the viewport")] {
                    ui.strong(key); ui.label(action); ui.end_row();
                }
            });
            ui.separator();
            ui.label("Import self-contained .glb files with embedded buffers and textures.\nFiles stay on your device. Reloading clears the workspace.");
            ui.label("This first version is for importing and inspecting models; mesh editing comes later.");
        });
    }
    if toggle_projection {
        state.orthographic = !state.orthographic;
        dolly.write(DollyZoomTrigger {
            camera: camera_entity,
            target_projection: if state.orthographic {
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                        viewport_height: 1.0,
                    },
                    ..OrthographicProjection::default_3d()
                })
            } else {
                Projection::Perspective(PerspectiveProjection::default())
            },
        });
    }
    if let Some(direction) = facing {
        look.write(LookToTrigger {
            camera: camera_entity,
            target_facing_direction: direction.as_dvec3(),
            target_up_direction: if direction.y < -0.5 {
                DVec3::NEG_Z
            } else if direction.y > 0.5 {
                DVec3::Z
            } else {
                DVec3::Y
            },
        });
    }
    if home {
        controller.end_move();
        controller.current_motion = motion::CurrentMotion::Stationary;
        *transform = Transform::from_xyz(9.0, 7.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y);
        controller.last_anchor_depth = -transform.translation.length() as f64;
        *projection = Projection::Perspective(PerspectiveProjection::default());
        state.orthographic = false;
    }
    if frame {
        let roots: Vec<Entity> = state
            .selected
            .map(|e| vec![e])
            .unwrap_or_else(|| imports.models.iter().filter_map(|m| m.root).collect());
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for root in roots {
            if let Some((a, b)) = viewport::bounds(root, &children, &meshes) {
                min = min.min(a);
                max = max.max(b);
            }
        }
        if min.is_finite() {
            viewport::frame_bounds(
                min,
                max,
                &mut transform,
                &mut projection,
                &mut controller,
                state.aspect(),
            );
        }
    }
    if let Some(index) = remove {
        let model = imports.models.remove(index);
        if let Some(root) = model.root {
            commands.entity(root).despawn();
        }
        files.0.remove_asset(std::path::Path::new(&model.path));
        state.selected = None;
    }
    // Convert egui points into physical pixels; this also follows egui zoom and HiDPI.
    let pixels = ctx.pixels_per_point();
    let position = UVec2::new(
        (rect.min.x.max(0.0) * pixels) as u32,
        (rect.min.y.max(0.0) * pixels) as u32,
    );
    let total = UVec2::new(window.physical_width(), window.physical_height());
    let available = total.saturating_sub(position);
    let size = UVec2::new(
        (rect.width().max(1.0) * pixels) as u32,
        (rect.height().max(1.0) * pixels) as u32,
    )
    .min(available);
    if size.x > 0 && size.y > 0 {
        camera.viewport = Some(Viewport {
            physical_position: position,
            physical_size: size,
            ..default()
        });
    }
    Ok(())
}

fn tree(
    ui: &mut Ui,
    entity: Entity,
    names: &Query<(Option<&Name>, Has<Mesh3d>)>,
    children: &Query<&Children>,
    selected: &mut Option<Entity>,
    root: bool,
    depth: usize,
) {
    if depth > 128 {
        ui.label("Hierarchy depth limit reached");
        return;
    }
    let (name, mesh) = names.get(entity).unwrap_or((None, false));
    let label = name
        .map(Name::as_str)
        .unwrap_or(if mesh { "Mesh" } else { "Node" });
    if let Ok(kids) = children.get(entity) {
        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id(entity),
            root,
        )
        .show_header(ui, |ui| {
            if ui
                .selectable_label(*selected == Some(entity), label)
                .clicked()
            {
                *selected = Some(entity);
            }
        })
        .body(|ui| {
            for child in kids.iter() {
                tree(ui, child, names, children, selected, false, depth + 1);
            }
        });
    } else if ui
        .selectable_label(*selected == Some(entity), label)
        .clicked()
    {
        *selected = Some(entity);
    }
}

fn inside_polygon(point: Pos2, polygon: &[Pos2]) -> bool {
    let mut sign = 0.0_f32;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if cross.abs() < 0.001 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if sign != cross.signum() {
            return false;
        }
    }
    true
}

/// Render the cube from the inverse camera rotation, so every face follows the
/// real viewport orientation, including while the controller animates a snap.
fn orientation_cube(ui: &mut Ui, rotation: Quat) -> Option<Vec3> {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(164.0, 142.0), egui::Sense::click());
    let center = rect.center();
    let view = rotation.inverse();
    let project = |v: Vec3| {
        let p = view * v;
        center + egui::vec2(p.x, -p.y) * 33.0
    };
    let specs = [
        (Vec3::Y, Vec3::X, Vec3::Z, "TOP"),
        (Vec3::NEG_Y, Vec3::X, Vec3::NEG_Z, "BOTTOM"),
        (Vec3::Z, Vec3::X, Vec3::Y, "FRONT"),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, "BACK"),
        (Vec3::X, Vec3::NEG_Z, Vec3::Y, "RIGHT"),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y, "LEFT"),
    ];
    let mut faces: Vec<_> = specs
        .into_iter()
        .filter(|(n, ..)| (view * *n).z > 0.001)
        .collect();
    faces.sort_by(|a, b| (view * a.0).z.total_cmp(&(view * b.0).z));
    let pointer = response.hover_pos();
    let mut clicked = None;
    for (normal, u, v, label) in faces {
        let polygon: Vec<Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|(x, y)| project(normal + u * *x + v * *y))
            .collect();
        let hovered = pointer.is_some_and(|p| inside_polygon(p, &polygon));
        let shade = (view * normal).z;
        let fill = if hovered {
            ACCENT
        } else {
            Color32::from_gray((60.0 + shade * 45.0) as u8)
        };
        ui.painter().add(egui::Shape::convex_polygon(
            polygon,
            fill,
            Stroke::new(1.5, Color32::from_rgb(27, 32, 39)),
        ));
        if shade > 0.25 {
            ui.painter().text(
                project(normal),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                if hovered {
                    Color32::from_rgb(20, 36, 28)
                } else {
                    Color32::from_rgb(221, 229, 239)
                },
            );
        }
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if response.clicked() {
                clicked = Some(-normal);
            }
        }
    }
    clicked
}
