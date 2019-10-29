use clap::{App, Arg};
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
    static ref KEY_TABLE_INDEX: RwLock<usize> = RwLock::new(0);
    static ref KEY_CODE: RwLock<Key> = RwLock::new(Key::F5);
}

static KEY_TABLE: [[Key; 2]; 2] = [
    [Key::PageUp, Key::PageDown],
    [Key::LeftArrow, Key::RightArrow],
];

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
            let target_key: Key;
            let current_key: Key;
            {
                let key_table_index = KEY_TABLE_INDEX.read().unwrap();
                target_key = KEY_TABLE[*key_table_index][1];

                let key_code = KEY_CODE.read().unwrap();
                current_key = *key_code;
            }
            if current_key != target_key {
                info!("{:?}", target_key);
                let mut key_code = KEY_CODE.write().unwrap();
                *key_code = target_key;
            }
        }
        _ => {
            let target_key: Key;
            let current_key: Key;
            {
                let key_table_index = KEY_TABLE_INDEX.read().unwrap();
                target_key = KEY_TABLE[*key_table_index][0];

                let key_code = KEY_CODE.read().unwrap();
                current_key = *key_code;
            }
            if current_key != target_key {
                info!("{:?}", target_key);
                let mut key_code = KEY_CODE.write().unwrap();
                *key_code = target_key;
            }
        }
    }

    Ok(())
}

fn main() {
    env_logger::init();
    let app = App::new("cubekey")
        .version("0.0.1")
        .arg(Arg::with_name("lr").help("LR key mode").long("lr"));
    let matches = app.get_matches();
    if matches.is_present("lr") {
        println!("LR key mode");
        let mut key_table_index = KEY_TABLE_INDEX.write().unwrap();
        *key_table_index = 1;
    } else {
        println!("Page key mode");
    }

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
