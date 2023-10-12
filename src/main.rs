use bevy::prelude::*;

mod mischief;

use mischief::{MischiefPlugin, poll_events, MischiefSession, MischiefEvent, MischiefEventData};

// Making a game with Bevy + Mischief
// Specifically, a game where you control two ends of a rope with two mice
// Candy falls from the top of the screen, and you have to catch it with the rope
// You can move the rope ends independently, but you can't move them too far apart
// You must deposit the candy in a receptacle on one side of the screen

// First steps:
// Make two mouse cursors that you can move around (done)
// Assign each cursor to a hand (i.e. click left mouse button to assign to left hand, right mouse button to assign to right hand)
// Make two rigid bodies that fall from the top of the screen
// Make the bodies dangle from the cursors

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MischiefPlugin)
        .add_systems(Startup, (spawn_camera, spawn_cursors))
        .add_systems(Update, move_cursors.after(poll_events))
        .run();
}

#[derive(Component)]
struct Cursor(u32);

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_cursors(mut commands: Commands, mouse_session: NonSend<MischiefSession>) {
    for device in &mouse_session.session.devices {
        commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(10.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
            Cursor(device.id)
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
