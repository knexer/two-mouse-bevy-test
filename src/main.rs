use manymouse::ManyMouse;

mod manymouse;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let manymouse = ManyMouse::init()?;
    let num_devices = manymouse.devices.len();
    println!("num_devices: {}", num_devices);

    for i in 0..num_devices {
        println!("name: {:?}", manymouse.devices[i].name);
    }

    let mut num_events = 0;
    while num_events < 100 {
        if let Some(event) = manymouse.poll_event()? {
            println!("event: {:?}", event);
            num_events += 1;
        }
    }

    Ok(())
}
