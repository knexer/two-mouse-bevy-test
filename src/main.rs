use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
    window::WindowResolution,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use link::LinkPlugin;

mod link;
mod mischief;
mod util;

// TODO: Create a main menu

const PIXELS_PER_METER: f32 = 100.0;
pub const BACKGROUND_COLOR: Color = Color::rgb(64.0 / 255.0, 67.0 / 255.0, 78.0 / 255.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LinkPlugin)
        .add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::Grave)))
        .add_systems(
            Update,
            toggle_os_cursor.run_if(input_just_pressed(KeyCode::Grave)),
        )
        .add_systems(
            Startup,
            (size_window, spawn_camera, toggle_os_cursor).chain(),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn size_window(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    let scale_factor = window.scale_factor() as f32;
    window.resolution = WindowResolution::new(1600.0 * scale_factor, 900.0 * scale_factor)
        .with_scale_factor_override(scale_factor as f64);
    window.position.center(MonitorSelection::Current);
}

fn toggle_os_cursor(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    let window_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    window.set_cursor_position(Some(window_center));
    window.cursor.visible = !window.cursor.visible;
    window.cursor.grab_mode = match window.cursor.visible {
        true => bevy::window::CursorGrabMode::None,
        false => bevy::window::CursorGrabMode::Locked,
    };
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            far: 1000.,
            near: -1000.,
            scale: 1.0 / PIXELS_PER_METER,
            ..default()
        },
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::Custom(BACKGROUND_COLOR),
        },
        ..default()
    });
}
