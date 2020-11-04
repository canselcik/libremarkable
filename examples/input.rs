use libremarkable::framebuffer::common::{MTHEIGHT, MTWIDTH, WACOMHEIGHT, WACOMWIDTH};
use libremarkable::input::{ev::EvDevContext, scan::SCAN, InputDevice, InputEvent};
use std::sync::mpsc::channel;

fn main() {
    // Measure start time
    let start = std::time::SystemTime::now();

    // Display paths for InputDevices
    for device in [
        InputDevice::GPIO,
        InputDevice::Multitouch,
        InputDevice::Wacom,
    ]
    .iter()
    {
        eprintln!("{:?} is {:?}", SCAN.get_path(*device), device);
    }

    eprintln!("Multitouch resolution: {}x{}", *MTWIDTH, *MTHEIGHT);
    eprintln!("Wacom resolution: {}x{}", *WACOMWIDTH, *WACOMHEIGHT);

    // Send all input events to input_rx
    let (input_tx, input_rx) = channel::<InputEvent>();
    EvDevContext::new(InputDevice::GPIO, input_tx.clone()).start();
    EvDevContext::new(InputDevice::Multitouch, input_tx.clone()).start();
    EvDevContext::new(InputDevice::Wacom, input_tx).start();

    // Output measurement of start time
    eprintln!("Opened input devices in {:?}", start.elapsed().unwrap());

    eprintln!("Waiting for input events...");
    while let Ok(event) = input_rx.recv() {
        println!("{:?}", event);
    }
    eprintln!("All event loops were closed?!?");
}
