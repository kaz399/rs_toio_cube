use core_cube::win10::*;
use ctrlc;
use enigo::*;
use env_logger;
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;

lazy_static! {
    static ref KEY_CODE: RwLock<Key> = RwLock::new(Key::PageDown);
}

fn button_notify(
    _sender: *mut CoreCubeNotifySender,
    arg: *mut CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("button status changed {:?}", data);
    if data[1] == 0x80 {
        let mut enigo = Enigo::new();
        let key_code = KEY_CODE.read().unwrap();
        enigo.key_down(*key_code);
        info!("send keycode {:?}", key_code);
    }
    Ok(())
}

fn sensor_information_notify(
    _sender: *mut CoreCubeNotifySender,
    arg: *mut CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("sensor information status changed {:?}", data);
    match data[1] {
        0x01 => {
            let current_key: Key;
            {
                let key_code = KEY_CODE.read().unwrap();
                current_key = *key_code;
            }
            if current_key != Key::PageDown {
                info!("PageDown");
                let mut key_code = KEY_CODE.write().unwrap();
                *key_code = Key::PageDown;
            }
        }
        _ => {
            let current_key: Key;
            {
                let key_code = KEY_CODE.read().unwrap();
                current_key = *key_code;
            }
            if current_key != Key::PageUp {
                info!("PageUp");
                let mut key_code = KEY_CODE.write().unwrap();
                *key_code = Key::PageUp;
            }
        }
    }

    Ok(())
}

fn main() {
    env_logger::init();
    let dev_list = get_ble_devices().unwrap();
    assert_ne!(dev_list.len(), 0);

    let mut cube = CoreCubeBLE::new("Cube1".to_string());
    let mut connected = false;
    for device_info in &dev_list {
        info!("Searching cube: {:?}", device_info);
        let result = cube.connect(device_info);
        match result.unwrap() {
            true => (),
            false => continue,
        }
        let result = cube.read(CoreCubeUuidName::SensorInfo);
        match result {
            Ok(_) => {
                connected = true;
                break;
            }
            Err(_) => continue,
        }
    }

    if connected == false {
        error!("No cubes!");
        return;
    }

    // let result = cube.write(
    //     CoreCubeUuidName::MotorCtrl,
    //     &vec![0x02, 0x01, 0x01, 0x64, 0x02, 0x02, 0x64, 0xff],
    // );
    // assert_eq!(result.unwrap(), true);

    let result = cube.register_norify(CoreCubeUuidName::ButtonInfo, button_notify);
    let button_handler = result.unwrap();

    let result = cube.register_norify(CoreCubeUuidName::SensorInfo, sensor_information_notify);
    let sensor_handler = result.unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {}

    let result = button_handler.unregister();
    assert_eq!(result.unwrap(), true);

    let result = sensor_handler.unregister();
    assert_eq!(result.unwrap(), true);
}
