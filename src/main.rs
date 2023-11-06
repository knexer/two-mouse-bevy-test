use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
    window::WindowResolution,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use link::LinkPlugin;

mod gameplay;
mod link;
mod mischief;
mod player;
mod spawn_level;
mod util;

// MVP brief features:

// You control two ends of a physics-simulated rope with two mice.
// You use the rope to sort green circles and purple squares into two containers.
// 10 shapes will fall from above the screen over the course of the level.
// Your score is how many shapes you sorted correctly minus how many you sorted incorrectly.
// Shapes may also fall out the bottom of the screen, which doesn't penalize your score.
// The game ends when all 10 shapes have fallen.
// Click any mouse button to start a new game.

// MVP is in place! Polish time.

// Polish:
// Sound effects!
// Spawn shapes in more interesting ways. Randomized params, spawn in waves, spawn in patterns.
// Round the rest of the corners on the right side of the level.
// Visual polish on the level shapes.
// Add drop shadows to shapes and cursor/chain.
// Improve the game over screen layout.
// Add left and right mouse button images to the title/setup screen.

// Done polish:
// Differentiate left vs right cursors visually. (done)
// Pick a nicer color palette and recolor everything with it. (done)
// Add a title screen shown during AppState::Init. (done)
// Add game over screen shown during AppState::GameOver. (done)
// Increase intensity over time. (done)
// Two shape patterns (sequence and shotgun). (done)

// Bugs:
// - Window resolution doesn't seem to be working as I expect it to.

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
