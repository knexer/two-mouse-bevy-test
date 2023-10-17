use std::time::Duration;

use bevy::{
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::WindowResolution,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;
use player::PlayerPlugin;
use rand::Rng;
use spawn_level::{Layer, SHAPE_ALIVE_REGION, SHAPE_SPAWN_REGION};

use crate::spawn_level::{LEFT_SCORE_REGION, RIGHT_SCORE_REGION};

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
// Wait to start the game until both cursors are assigned.
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
// - Sometimes the game freezes, maybe physics related? Happens sometimes at game start, or when things spawn on top of each other.
// - Window resolution doesn't seem to be working as I expect it to.

const PIXELS_PER_METER: f32 = 100.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_plugins(PhysicsPlugins::new(FixedUpdate))
        .insert_resource(SubstepCount(20))
        .add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::Grave)))
        .add_systems(
            Update,
            toggle_os_cursor.run_if(input_just_pressed(KeyCode::Grave)),
        )
        .add_systems(
            Startup,
            (size_window, spawn_camera, toggle_os_cursor).chain(),
        )
        .add_systems(Startup, spawn_level::spawn_level)
        .add_systems(Startup, configure_shapes)
        .add_systems(Update, (spawn_shapes, despawn_shapes))
        .insert_resource(Score::default())
        .add_systems(Update, (update_score, display_score).chain())
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
        ..default()
    });
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Square,
    Circle,
}

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shape::Square => write!(f, "Square"),
            Shape::Circle => write!(f, "Circle"),
        }
    }
}

#[derive(Component)]
struct ShapeConfig {
    mesh: Mesh2dHandle,
    material: Handle<ColorMaterial>,
    collider: Collider,
    shape: Shape,
}

fn configure_shapes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let default_size = 0.25;
    commands.spawn((
        ShapeConfig {
            mesh: meshes
                .add(
                    shape::Quad {
                        size: Vec2::splat(default_size),
                        ..default()
                    }
                    .into(),
                )
                .into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            collider: Collider::cuboid(default_size, default_size),
            shape: Shape::Square,
        },
        Name::new("SquareConfig"),
    ));
    commands.spawn((
        ShapeConfig {
            mesh: meshes
                .add(
                    shape::Circle {
                        radius: default_size / 2.0,
                        ..default()
                    }
                    .into(),
                )
                .into(),
            material: materials.add(ColorMaterial::from(Color::GREEN)),
            collider: Collider::ball(default_size / 2.0),
            shape: Shape::Circle,
        },
        Name::new("CircleConfig"),
    ));
    commands.insert_resource(ShapeTimer(Timer::from_seconds(2.0, TimerMode::Once)));
}

#[derive(Resource)]
struct ShapeTimer(Timer);

fn spawn_shapes(
    mut commands: Commands,
    mut shape_configs: Query<&mut ShapeConfig>,
    mut shape_timer: ResMut<ShapeTimer>,
    time: Res<Time>,
) {
    if !shape_timer
        .0
        .tick(Duration::from_secs_f32(time.delta_seconds()))
        .just_finished()
    {
        return;
    }
    let mut rng = rand::thread_rng();
    // Pick a random shape config
    let shape_configs = shape_configs.iter_mut().collect::<Vec<_>>();
    let shape_config = &shape_configs[rng.gen_range(0..shape_configs.len())];

    let x = rng.gen_range(SHAPE_SPAWN_REGION.min.x..SHAPE_SPAWN_REGION.max.x);
    let y = rng.gen_range(SHAPE_SPAWN_REGION.min.y..SHAPE_SPAWN_REGION.max.y);
    commands.spawn((
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(x, y, 0.0),
            mesh: shape_config.mesh.clone(),
            material: shape_config.material.clone(),
            ..default()
        },
        RigidBody::Dynamic,
        shape_config.collider.clone(),
        shape_config.shape.clone(),
        CollisionLayers::new([Layer::Shapes], [Layer::Rope, Layer::Level, Layer::Shapes]),
        Name::new(shape_config.shape.to_string()),
    ));

    shape_timer
        .0
        .set_duration(Duration::from_secs_f32(rng.gen_range(1.0..3.0)));
    shape_timer.0.reset();
}

fn despawn_shapes(mut commands: Commands, mut shapes: Query<(Entity, &Transform), With<Shape>>) {
    for (entity, transform) in shapes.iter_mut() {
        if !SHAPE_ALIVE_REGION.contains(transform.translation.truncate()) {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Resource, Default)]
struct Score {
    left: i32,
    right: i32,
}

fn update_score(mut score: ResMut<Score>, shapes: Query<(&Transform, &Shape)>) {
    score.left = 0;
    score.right = 0;
    for (transform, shape) in shapes.iter() {
        if LEFT_SCORE_REGION.contains(transform.translation.truncate()) {
            match shape {
                Shape::Square => score.left += 1,
                Shape::Circle => score.left -= 1,
            }
        } else if RIGHT_SCORE_REGION.contains(transform.translation.truncate()) {
            match shape {
                Shape::Square => score.right -= 1,
                Shape::Circle => score.right += 1,
            }
        }
    }
}

#[derive(Component)]
pub enum ScoreDisplay {
    Left,
    Right,
}

fn display_score(score: Res<Score>, mut displays: Query<(&mut Text, &ScoreDisplay)>) {
    for (mut text, display) in displays.iter_mut() {
        text.sections[0].value = match display {
            ScoreDisplay::Left => format!("{}", score.left),
            ScoreDisplay::Right => format!("{}", score.right),
        };
    }
}
