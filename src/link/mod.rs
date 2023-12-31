use crate::mischief::{MischiefEvent, MischiefEventData};
use crate::util::cleanup_system;
use bevy::{
    core_pipeline::clear_color::ClearColorConfig, input::common_conditions::input_just_pressed,
    prelude::*, window::WindowResolution,
};
use bevy_xpbd_2d::prelude::*;
use gameplay::GameplayPlugin;
use player::{AttachState, PlayerPlugin};
use spawn_level::{SpawnPlugin, SpawnState};

mod gameplay;
mod player;
mod spawn_level;

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
pub const LEFT_COLOR: Color = Color::rgb(17.0 / 255.0, 159.0 / 255.0, 166.0 / 255.0);
pub const RIGHT_COLOR: Color = Color::rgb(226.0 / 255.0, 101.0 / 255.0, 60.0 / 255.0);
pub const TEXT_COLOR: Color = Color::rgb(215.0 / 255.0, 217.0 / 255.0, 206.0 / 255.0);
pub const BAD_COLOR: Color = Color::rgb(229.0 / 255.0, 39.0 / 255.0, 36.0 / 255.0);

pub struct LinkPlugin;

impl Plugin for LinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlayerPlugin)
            .add_plugins(SpawnPlugin)
            .add_plugins(GameplayPlugin)
            .add_plugins(PhysicsPlugins::new(FixedUpdate))
            .insert_resource(SubstepCount(20))
            .add_systems(
                Update,
                toggle_os_cursor.run_if(input_just_pressed(KeyCode::Grave)),
            )
            .add_systems(
                Startup,
                (size_window, spawn_camera, toggle_os_cursor).chain(),
            )
            .add_systems(Update, bevy::window::close_on_esc)
            .add_state::<AppState>()
            .add_systems(Update, start_playing.run_if(in_state(AppState::Init)))
            .add_systems(OnExit(AppState::Init), cleanup_system::<DespawnOnExitInit>)
            .add_systems(Update, start_new_game.run_if(in_state(AppState::GameOver)))
            .add_systems(
                OnExit(AppState::GameOver),
                cleanup_system::<DespawnOnExitGameOver>,
            );
    }
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

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Init,
    Playing,
    GameOver,
}

fn start_playing(
    spawn_state: Res<State<SpawnState>>,
    attach_state: Res<State<AttachState>>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    if spawn_state.get() == &SpawnState::Done && attach_state.get() == &AttachState::Attached {
        app_state.set(AppState::Playing);
    }
}

fn start_new_game(
    mut app_state: ResMut<NextState<AppState>>,
    mut mischief_events: EventReader<MischiefEvent>,
) {
    for event in mischief_events.iter() {
        if let MischiefEventData::Button {
            button: _,
            pressed: true,
        } = event.event_data
        {
            app_state.set(AppState::Playing);
        }
    }
}

#[derive(Component)]
pub struct DespawnOnExitInit;

#[derive(Component)]
pub struct DespawnOnExitGameOver;
