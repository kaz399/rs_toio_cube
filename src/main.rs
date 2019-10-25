use std::thread;
use std::time;

use core_cube::win10::*;

fn button_handler(
    sender: *mut GattCharacteristic,
    arg: *mut GattValueChangedEventArgs,
) -> Result<()> {
    println!("button status changed {:?} {:?}", sender, arg);
    Ok(())
}

fn main() {
    let dev_list = get_ble_devices().unwrap();
    assert_ne!(dev_list.len(), 0);
    let device_info = &dev_list[0];

    let mut cube = CoreCubeBLE::new("Cube1".to_string());
    let result = cube.connect(device_info);
    assert_eq!(result.unwrap(), true);

    let result = cube.read(CoreCubeUuidName::SensorInfo);
    println!("{:?}", result.unwrap());

    let result = cube.write(
        CoreCubeUuidName::MotorCtrl,
        &vec![0x02, 0x01, 0x01, 0x64, 0x02, 0x02, 0x64, 0xff],
    );
    assert_eq!(result.unwrap(), true);

    println!("Notify test");
    let result = cube.register_norify(CoreCubeUuidName::ButtonInfo, button_handler);
    let notify_handler = result.unwrap();

    println!("sleep2");
    thread::sleep(time::Duration::from_secs(5));
    println!("wake up");
    let result = notify_handler.unregister();
    assert_eq!(result.unwrap(), true);
    println!("Hello, world!");
}
