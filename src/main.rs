use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod mischief;

use mischief::{MischiefPlugin, poll_events, MischiefEvent, MischiefEventData};

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
// Make a single rope that connects the two cursors

const PIXELS_PER_METER: f32 = 100.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MischiefPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(PIXELS_PER_METER))
        .add_plugins(RapierDebugRenderPlugin::default())
        // .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, (spawn_camera, hide_os_cursor))
        .add_systems(Startup, spawn_test_bodies)
        .add_systems(Startup, spawn_cursors)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, attach_cursors)
        .add_systems(Update, move_cursors.after(poll_events))
        .add_systems(Update, apply_cursor_force.after(move_cursors)) // TODO update order relative to physics?
        .run();
}

fn hide_os_cursor(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    let window_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    window.set_cursor_position(Some(window_center));
    window.cursor.visible = false;
    window.cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
}

#[derive(Component)]
struct Cursor(Option<u32>);

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
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
            MischiefEventData::Button { button: 0, pressed: true } => {
                if left_cursor_device == None && right_cursor_device != Some(event.device) {
                    let mut cursor = left_cursors.single_mut();
                    cursor.0 = Some(event.device);
                }
            },
            MischiefEventData::Button { button: 1, pressed: true } => {
                if right_cursor_device == None && left_cursor_device != Some(event.device) {
                    let mut cursor = right_cursors.single_mut();
                    cursor.0 = Some(event.device);
                }
            },
            _ => {}
        }
    }
}

fn spawn_cursors(
    mut commands: Commands,
) {
    spawn_cursor::<LeftCursor>(&mut commands, Vec2::new(-200.0, 0.0));
    spawn_cursor::<RightCursor>(&mut commands, Vec2::new(200.0, 0.0));
}

fn spawn_cursor<T>(commands: &mut Commands, start_pos: Vec2) where T: Component + Default {
    let cursor_size = 40.0;
    let mut parent_id = commands.spawn(
        (
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
                max_integral_error: 10.0,
                prev_error: Vec2::ZERO,
                integral_error: Vec2::ZERO,
            },
            ReadMassProperties::default(),
            Velocity::default(),
            ExternalImpulse::default(),
            LockedAxes::ROTATION_LOCKED,
            Collider::cuboid(cursor_size / 2.0, cursor_size / 2.0),
            Cursor(None),
            T::default()
        )).id();

    let body_size = 10.0;
    let start_shift = 10.0 + cursor_size / 2.0;
    let shift= 10.0 + body_size / 2.0;
    let final_shift = 20.0;
    const NUM_ROPES:u32 = 4;
    for i in 0..NUM_ROPES {
        let dx = start_shift + i as f32 * shift + match i + 1 { NUM_ROPES => final_shift - shift, _ => 0.0 };

        let rope = RopeJointBuilder::new()
            .local_anchor2(Vec2::new(0.0, 0.0))
            .limits([0.0, match i + 1 { 1 => start_shift, NUM_ROPES => final_shift, _ => shift }]);
        let joint = ImpulseJoint::new(parent_id, rope);

        let body_size = match i + 1 { NUM_ROPES => 30.0, _ => body_size };

        parent_id = commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(start_pos.x + dx, start_pos.y, 0.0),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(body_size)),
                    ..default()
                },
                ..default()
            },
            RigidBody::Dynamic,
            Collider::cuboid(body_size / 2.0, body_size / 2.0),
            joint,
        )).id();
    }
}

fn spawn_test_bodies(mut commands: Commands) {
    let positions = vec![
        Vec2::new(-200.0, 300.0),
        Vec2::new(200.0, 300.0),
    ];
    let body_size = 20.0;
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
            Collider::cuboid(body_size / 2.0, body_size / 2.0),
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
                        target_velocity.0 += Vec2::new(x as f32, -y as f32) / time.delta_seconds();
                    },
                    MischiefEventData::Disconnect => {
                        panic!("Mouse disconnected");
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Component)]
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

fn apply_cursor_force(mut cursors: Query<(&TargetVelocity, &mut PIDController, &ReadMassProperties, &Velocity, &mut ExternalImpulse)>, time: Res<Time>) {
    for (target_velocity, mut pd, mass, velocity, mut force) in cursors.iter_mut() {
        let error = target_velocity.0 - velocity.linvel;
        pd.integral_error += error * time.delta_seconds();
        pd.integral_error = pd.integral_error.clamp_length_max(pd.max_integral_error);
        // let d_error = (error - pd.prev_error) / time.delta_seconds();
        let u_pd = pd.p * error + pd.i * pd.integral_error;// + pd.d * d_error;

        // Multiply by mass to get impulse from difference in velocity
        force.impulse = mass.0.mass * u_pd;

        pd.prev_error = error;
    }
}
