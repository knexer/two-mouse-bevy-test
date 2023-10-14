use bevy::{
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;

mod mischief;

use mischief::{poll_events, MischiefEvent, MischiefEventData, MischiefPlugin};

// Making a game with Bevy + Mischief
// Specifically, a game where you control two ends of a rope with two mice
// Candy falls from the top of the screen, and you have to catch it with the rope
// You can move the rope ends independently, but you can't move them too far apart
// You must deposit the candy in a receptacle on one side of the screen

// Or... what if it's a mouth? Two-mouse pacman controls???
// Could open and close the mouth (by moving cursors together and apart) to move forward,
// and then turn left and right by moving both cursors left and right.
// BONKERS controls lol

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

const PIXELS_PER_METER: f32 = 100.0;

fn main() {
    App::new()
        .register_type::<TargetVelocity>()
        .add_plugins(DefaultPlugins)
        .add_plugins(MischiefPlugin)
        .add_plugins(PhysicsPlugins::new(FixedUpdate))
        .add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::Grave)))
        .add_systems(
            Update,
            toggle_os_cursor.run_if(input_just_pressed(KeyCode::Grave)),
        )
        .add_systems(Startup, (spawn_camera, toggle_os_cursor))
        .add_systems(Startup, spawn_test_bodies)
        .add_systems(Startup, spawn_cursors)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, attach_cursors)
        .add_systems(
            Update,
            move_cursors
                .after(poll_events)
                .run_if(input_toggle_active(true, KeyCode::Grave)),
        )
        .add_systems(FixedUpdate, apply_cursor_force.before(PhysicsSet::Prepare))
        .run();
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

#[derive(Component)]
struct Cursor(Option<u32>);

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

#[derive(Component, Default)]
struct LeftCursor;

#[derive(Component, Default)]
struct RightCursor;

fn attach_cursors(
    mut mouse_events: EventReader<MischiefEvent>,
    mut left_cursors: Query<&mut Cursor, (With<LeftCursor>, Without<RightCursor>)>,
    mut right_cursors: Query<&mut Cursor, (With<RightCursor>, Without<LeftCursor>)>,
) {
    let left_cursor_device = left_cursors.single().0;
    let right_cursor_device = right_cursors.single().0;
    for event in mouse_events.iter() {
        match event.event_data {
            MischiefEventData::Button {
                button: 0,
                pressed: true,
            } => {
                if left_cursor_device == None && right_cursor_device != Some(event.device) {
                    let mut cursor = left_cursors.single_mut();
                    cursor.0 = Some(event.device);
                }
            }
            MischiefEventData::Button {
                button: 1,
                pressed: true,
            } => {
                if right_cursor_device == None && left_cursor_device != Some(event.device) {
                    let mut cursor = right_cursors.single_mut();
                    cursor.0 = Some(event.device);
                }
            }
            _ => {}
        }
    }
}

#[derive(PhysicsLayer)]
enum Layer {
    Rope,
    Other,
}

fn spawn_cursors(mut commands: Commands) {
    let left_pos = Vec2::new(-3.0, 0.0);
    let right_pos = Vec2::new(3.0, 0.0);
    let left_cursor = spawn_cursor::<LeftCursor>(&mut commands, left_pos, None, "Left Cursor");
    let last_rope = spawn_rope(&mut commands, left_pos, right_pos, 20, left_cursor);
    spawn_cursor::<RightCursor>(&mut commands, right_pos, Some(last_rope), "Right Cursor");
}

fn spawn_cursor<T>(
    commands: &mut Commands,
    start_pos: Vec2,
    connect_to: Option<(Entity, Vec2)>,
    name: &str,
) -> Entity
where
    T: Component + Default,
{
    let cursor_size = 0.4;
    let cursor_id = commands
        .spawn((
            SpriteBundle {
                transform: Transform::from_xyz(start_pos.x, start_pos.y, 0.0),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(cursor_size)),
                    ..default()
                },
                ..default()
            },
            RigidBody::Dynamic,
            TargetVelocity(Vec2::ZERO),
            PIDController {
                p: 1.0,
                i: 1.0,
                d: 0.0,
                max_integral_error: 0.2,
                prev_error: Vec2::ZERO,
                integral_error: Vec2::ZERO,
            },
            LinearVelocity::default(),
            ExternalForce::default().with_persistence(false),
            LockedAxes::ROTATION_LOCKED,
            Collider::cuboid(cursor_size, cursor_size),
            CollisionLayers::new([Layer::Rope], [Layer::Other]),
            Cursor(None),
            T::default(),
            Name::new(name.to_owned()),
        ))
        .id();

    if let Some((entity, prev_anchor)) = connect_to {
        let rope_joint = RevoluteJoint::new(entity, cursor_id)
            .with_local_anchor_1(prev_anchor)
            .with_local_anchor_2(Vec2::new(0.0, 0.0));
        commands.spawn(rope_joint);
    };

    return cursor_id;
}

fn spawn_rope(
    commands: &mut Commands,
    start_pos: Vec2,
    end_pos: Vec2,
    num_segments: u32,
    parent_id: Entity,
) -> (Entity, Vec2) {
    // Total width of n segments: width = (n + 1) * GAP + n * body_size
    // Solving for body_size: body_size = (width - (n + 1) * GAP) / n
    const GAP: f32 = 0.05;
    let total_gap_width = (num_segments + 1) as f32 * GAP;
    let body_length = ((end_pos.x - start_pos.x) - total_gap_width) / num_segments as f32;
    const THICKNESS: f32 = 0.05;

    let mut prev_id = parent_id;
    let mut prev_anchor = Vec2::new(0.0, 0.0);
    for i in 0..num_segments {
        let dx = (i as f32 + 1.0) * GAP + (i as f32) * body_length;

        let current_id = commands
            .spawn((
                SpriteBundle {
                    transform: Transform::from_xyz(
                        start_pos.x + dx + body_length / 2.0,
                        start_pos.y,
                        0.0,
                    ),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(body_length, THICKNESS)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::cuboid(body_length, THICKNESS),
                CollisionLayers::new([Layer::Rope], [Layer::Other]),
                Name::new(format!("Rope segment {}", i)),
            ))
            .id();

        let rope_joint = RevoluteJoint::new(prev_id, current_id)
            .with_local_anchor_1(prev_anchor)
            .with_local_anchor_2(Vec2::new(-(body_length + GAP) / 2.0, 0.0));
        prev_anchor = Vec2::new((body_length + GAP) / 2.0, 0.0);
        commands.spawn(rope_joint);

        prev_id = current_id;
    }
    return (prev_id, prev_anchor);
}

fn spawn_test_bodies(mut commands: Commands) {
    let positions = vec![Vec2::new(-2.0, 3.0), Vec2::new(2.0, 3.0)];
    let body_size = 0.5;
    for position in positions {
        commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(position.x, position.y, 0.0),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(body_size)),
                    ..default()
                },
                ..default()
            },
            RigidBody::Dynamic,
            Collider::cuboid(body_size, body_size),
        ));
    }
}

fn move_cursors(
    mut mouse_events: EventReader<MischiefEvent>,
    mut cursor_query: Query<(&mut TargetVelocity, &Cursor)>,
    time: Res<Time>,
) {
    for (mut target_velocity, _) in cursor_query.iter_mut() {
        target_velocity.0 = Vec2::ZERO;
    }

    for event in mouse_events.iter() {
        for (mut target_velocity, cursor) in cursor_query.iter_mut() {
            if cursor.0 == Some(event.device) {
                match event.event_data {
                    MischiefEventData::RelMotion { x, y } => {
                        target_velocity.0 += Vec2::new(x as f32, -y as f32)
                            / (PIXELS_PER_METER * time.delta_seconds());
                    }
                    MischiefEventData::Disconnect => {
                        panic!("Mouse disconnected");
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Component, Reflect, Debug)]
struct TargetVelocity(Vec2);

#[derive(Component)]
struct PIDController {
    p: f32,
    i: f32,
    d: f32,
    max_integral_error: f32,
    integral_error: Vec2,
    prev_error: Vec2,
}

fn apply_cursor_force(
    mut cursors: Query<(
        &TargetVelocity,
        &mut PIDController,
        &Mass,
        &LinearVelocity,
        &mut ExternalForce,
    )>,
    time: Res<FixedTime>,
) {
    for (target_velocity, mut pd, mass, velocity, mut force) in cursors.iter_mut() {
        let error = target_velocity.0 - velocity.0;

        pd.integral_error += error * time.period.as_secs_f32();
        pd.integral_error = pd.integral_error.clamp_length_max(pd.max_integral_error);
        let d_error = (error - pd.prev_error) / time.period.as_secs_f32();
        let u_pd = pd.p * error + pd.i * pd.integral_error + pd.d * d_error;

        let applied_acceleration = u_pd / time.period.as_secs_f32();
        force.apply_force(mass.0 * applied_acceleration);

        pd.prev_error = error;
    }
}
