use std::thread;
use std::time;

use core_cube::win10::*;

fn button_notify(
    _sender: *mut CoreCubeNotifySender,
    arg: *mut CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    println!("button status changed {:?}", data);
    Ok(())
}

fn sensor_information_notify(
    _sender: *mut CoreCubeNotifySender,
    arg: *mut CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    println!("sensor information status changed {:?}", data);
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
    let result = cube.register_norify(CoreCubeUuidName::ButtonInfo, button_notify);
    let button_handler = result.unwrap();

    let result = cube.register_norify(CoreCubeUuidName::SensorInfo, sensor_information_notify);
    let sensor_handler = result.unwrap();

    println!("sleep");
    thread::sleep(time::Duration::from_secs(15));
    println!("wake up");

    let result = button_handler.unregister();
    assert_eq!(result.unwrap(), true);

    let result = sensor_handler.unregister();
    assert_eq!(result.unwrap(), true);

    println!("Hello, world!");
}
