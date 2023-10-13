use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

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
// Move the cursor with forces so it doesn't make the rope go crazy
// Make a single rope that connects the two cursors

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MischiefPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, (spawn_camera, hide_os_cursor))
        .add_systems(Startup, spawn_test_bodies)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, spawn_cursors.run_if(
            not(any_with_component::<LeftCursor>())
            .or_else(not(any_with_component::<RightCursor>()))))
        .add_systems(Update, move_cursors.after(poll_events))
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
struct Cursor(u32);

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component, Default)]
struct LeftCursor;

#[derive(Component, Default)]
struct RightCursor;

fn spawn_cursors(
    mut commands: Commands,
    mut mouse_events: EventReader<MischiefEvent>,
    left_cursors: Query<&Cursor, With<LeftCursor>>,
    right_cursors: Query<&Cursor, With<RightCursor>>,
) {
    for event in mouse_events.iter() {
        match event.event_data {
            MischiefEventData::Button { button: 0, pressed: true } => {
                if left_cursors.is_empty() && (right_cursors.is_empty() || right_cursors.single().0 != event.device) {
                    spawn_cursor::<LeftCursor>(&mut commands, Vec2::new(-200.0, 0.0), event.device);
                }
            },
            MischiefEventData::Button { button: 1, pressed: true } => {
                if right_cursors.is_empty() && (left_cursors.is_empty() || left_cursors.single().0 != event.device) {
                    spawn_cursor::<RightCursor>(&mut commands, Vec2::new(200.0, 0.0), event.device);
                }
            },
            _ => {}
        }
    }
}

fn spawn_cursor<T>(commands: &mut Commands, start_pos: Vec2, device: u32) where T: Component + Default {
    let cursor_size = 20.0;
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
            RigidBody::KinematicPositionBased,
            Collider::cuboid(cursor_size / 2.0, cursor_size / 2.0),
            Cursor(device),
            T::default()
        )).id();

    let body_size = 10.0;
    let shift = 20.0;
    for i in 0..10 {
        let dx = (i + 1) as f32 * shift;
        let rope = RopeJointBuilder::new()
            .local_anchor2(Vec2::new(0.0, 0.0))
            .limits([0.0, shift]);
        let joint = ImpulseJoint::new(parent_id, rope);

        let body_size = match i { 9 => 20.0, _ => body_size };

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
    mut cursor_query: Query<(&mut Transform, &Cursor)>
) {
    for event in mouse_events.iter() {
        for (mut transform, cursor) in cursor_query.iter_mut() {
            if cursor.0 == event.device {
                match event.event_data {
                    MischiefEventData::RelMotion { x, y } => {
                        transform.translation.x += x as f32;
                        transform.translation.y -= y as f32;
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
