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
use spawn_level::{SHAPE_ALIVE_REGION, SHAPE_SPAWN_REGION};

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

// Todo list:
// Spawn a purple square and a green circle at the top of the screen. (done)
// Spawn on a timer instead of at the start. (done)
// Randomize their params (size, position, velocity, etc.). (position done)
// Split out some modules. (done)
// Rework level layout - shapes fall in from offscreen, add containers for shapes on the sides, slope the floor towards a center drain. (done)
// Block the player from moving the rope outside the level.
// Pick a nice color palette and recolor everything with it.
// Round the rest of the corners on the right side of the level.
// Differentiate left vs right cursors visually.
// Add a score counter for each side.
// Wait to start the game until both cursors are assigned.

// Bugs:
// - Sometimes the game freezes, maybe physics related? Happens sometimes at game start, or when things spawn on top of each other.

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
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn size_window(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    window.resolution = WindowResolution::new(1600.0, 900.0);
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
