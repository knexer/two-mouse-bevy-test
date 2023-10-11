use bevy::prelude::*;

mod mischief;

use mischief::{MischiefPlugin, poll_events};
use mischief::manymouse_session::ManyMouseSession;

fn main() {
    test_manymouse();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MischiefPlugin)
        .add_systems(Update, print_events.after(poll_events))
        .run();
}

fn test_manymouse() {
    let session = ManyMouseSession::init().unwrap();
    println!("Found {} mice", session.devices.len());
    let mut num_events = 0;
    while num_events < 100 {
        if let Some(event) = session.poll_event().unwrap() {
            println!("{:?}", event);
            num_events += 1;
        }
    }
}

fn print_events(mut reader: EventReader<mischief::MischiefEvent>) {
    // println!("Events:");
    for event in reader.iter() {
        println!("{:?}", event);
    }
}
