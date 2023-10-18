use bevy::{
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
    window::WindowResolution,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;
use gameplay::GameplayPlugin;
use mischief::{MischiefEvent, MischiefEventData};
use player::{AttachState, PlayerPlugin};
use spawn_level::{SpawnPlugin, SpawnState};

mod gameplay;
mod mischief;
mod path;
mod player;
mod spawn_level;

// Making a game with Bevy + Mischief
// Specifically, a game where you control two ends of a rope with two mice.
// You manipulate other objects with the rope.

// First steps:
// Make two virtual cursors that you can move around (done)
// Assign each cursor to a hand (i.e. click LMB to assign to left hand, RMB to assign to right hand) (done)
// Capture and hide the OS cursor (done)
// Press escape to quit (done)
// Make two rigid bodies that fall from the top of the screen (done)
// Make the bodies dangle from the cursors (done)
// Make a rope of bodies that dangles from the cursors (done)
// Move the cursor with forces so it doesn't make the rope go crazy (done)
// Make a single rope that connects the two cursors (done! finally!)

// Okay, the basic platform is in place. Let's make a game!

// Revised plan:
// Two types of shapes fall from the top of the screen.
// One type should go to the left; the other to the right; they can also fall straight through and be gone.
// You get points for sorting correctly, lose points for sorting wrong, and miss out on points for letting them fall through.

// MVP:
// Spawn a purple square and a green circle at the top of the screen. (done)
// Spawn on a timer instead of at the start. (done)
// Randomize their params (size, position, velocity, etc.). (position done)
// Split out some modules. (done)
// Rework level layout - shapes fall in from offscreen, add containers for shapes on the sides, slope the floor towards a center drain. (done)
// Block the player from moving the rope outside the level. (done)
// Add a score counter for each side. (done)
// Wait to start the game until both cursors are assigned. (done)
// Add an end condition. A timer? A score threshold? A number of shapes?

// Polish:
// Sound effects!
// Show a title screen while waiting for the player to attach the cursors.
// Increase intensity over time.
// Spawn shapes in more interesting ways. Randomized params, spawn in waves, spawn in patterns.
// Differentiate left vs right cursors visually.
// Pick a nice color palette and recolor everything with it.
// Round the rest of the corners on the right side of the level.

// Bugs:
// - Window resolution doesn't seem to be working as I expect it to.

const PIXELS_PER_METER: f32 = 100.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_plugins(SpawnPlugin)
        .add_plugins(GameplayPlugin)
        .add_plugins(PhysicsPlugins::new(FixedUpdate))
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
        .add_state::<AppState>()
        .add_systems(Update, start_playing.run_if(in_state(AppState::Init)))
        .add_systems(Update, start_new_game.run_if(in_state(AppState::GameOver)))
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
