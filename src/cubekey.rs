use clap::{App, Arg};
use core_cube::win10::*;
use ctrlc;
use enigo::*;
use env_logger;
use lazy_static::lazy_static;
use log::{debug, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::{thread, time};

#[derive(Debug, Clone)]
struct RequestFromHandler {
    name: String,
    time: time::Instant,
    uuid: CoreCubeUuidName,
    data: Vec<u8>,
}

#[derive(Copy, Clone)]
enum KeyTableName {
    Page = 0,
    LR = 1,
    UD = 2,
}

lazy_static! {
    static ref KEY_TABLE_INDEX: RwLock<KeyTableName> = RwLock::new(KeyTableName::Page);
    static ref KEY_CODE: RwLock<Key> = RwLock::new(Key::F5);
    static ref PREV_PRESS_TIME: RwLock<Option<time::Instant>> = RwLock::new(None);
    static ref NOTIFY_RESULT: Mutex<Vec<RequestFromHandler>> = Mutex::new(Vec::new());
}

static KEY_TABLE: [[Key; 2]; 3] = [
    [Key::PageUp, Key::PageDown],
    [Key::LeftArrow, Key::RightArrow],
    [Key::UpArrow, Key::DownArrow],
];

fn button_notify(
    _sender: *mut CoreCubeNotifySender,
    arg: *mut CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("button status changed {:?}", data);
    match data[1] {
        0x00 => {
            let mut prev_press_time = PREV_PRESS_TIME.write().unwrap();
            match *prev_press_time {
                Some(x) => {
                    let elapsed_sec = x.elapsed().as_secs();
                    let elapsed_msec = x.elapsed().subsec_millis();
                    let key_code: Key;

                    if (elapsed_sec >= 1) || (elapsed_msec >= 800) {
                        key_code = Key::Home;

                        let notify_result_data = RequestFromHandler {
                            name: "ButtonNotify".to_string(),
                            time: time::Instant::now(),
                            uuid: CoreCubeUuidName::SoundCtrl,
                            data: vec![0x03, 0x01, 0x01, 0x05, 97, 0x80],
                        };
                        debug!("get lock");
                        let mut notify_result = NOTIFY_RESULT.lock().unwrap();
                        (*notify_result).push(notify_result_data.clone());
                        debug!("release lock");
                    } else {
                        key_code = *KEY_CODE.read().unwrap();
                    }
                    let mut engio = Enigo::new();
                    engio.key_down(key_code);
                    info!("send keycode {:?}", key_code);
                }
                None => {
                    *prev_press_time = Some(time::Instant::now());
                }
            }
        }
        0x80 => {
            let mut prev_press_time = PREV_PRESS_TIME.write().unwrap();
            *prev_press_time = Some(time::Instant::now());
        }
        _ => (),
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
                target_key = KEY_TABLE[*key_table_index as usize][1];

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
                target_key = KEY_TABLE[*key_table_index as usize][0];

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
        .arg(Arg::with_name("lr").help("LR arrow key mode").long("lr"))
        .arg(Arg::with_name("ud").help("UD arrow key mode").long("ud"));
    let matches = app.get_matches();
    if matches.is_present("lr") {
        println!("LR arrow key mode");
        let mut key_table_index = KEY_TABLE_INDEX.write().unwrap();
        *key_table_index = KeyTableName::LR;
    } else if matches.is_present("ud") {
        println!("UD arrow key mode");
        let mut key_table_index = KEY_TABLE_INDEX.write().unwrap();
        *key_table_index = KeyTableName::UD;
    } else {
        println!("Page key mode");
        let mut key_table_index = KEY_TABLE_INDEX.write().unwrap();
        *key_table_index = KeyTableName::Page;
    }

    let mut cube = CoreCubeBLE::new("Cube1".to_string());
    let mut connected = false;

    while connected == false {
        println!("search registered cubes");
        let dev_list = get_ble_devices().unwrap();
        assert_ne!(dev_list.len(), 0);

        for device_info in &dev_list {
            info!("Searching cube: {:?}", device_info);
            let result = cube.connect(device_info);
            match result.unwrap() {
                true => {
                    let result = cube.read(CoreCubeUuidName::BatteryInfo);
                    match result {
                        Ok(v) => {
                            connected = true;
                            println!("battery level {}%", v[0]);
                            break;
                        }
                        Err(_) => continue,
                    }
                }
                false => {
                    info!("search next cube");
                    continue;
                }
            }
        }
    }

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

    let tick = time::Duration::from_millis(50);

    let mut last_request = time::Instant::now();
    while running.load(Ordering::SeqCst) {
        // ensure lock term
        {
            let mut notify_result = NOTIFY_RESULT.lock().unwrap();
            let requests = &*notify_result;
            for request in requests {
                if request.time > last_request {
                    info!("Receive request {:?}", request);
                    last_request = request.time;
                    let result = cube.write(request.uuid, &request.data);
                    assert_eq!(result.unwrap(), true);
                }
            }
            (*notify_result).clear();
        }
        thread::sleep(tick);
    }

    let result = cube.write(
        CoreCubeUuidName::SoundCtrl,
        &vec![0x03, 0x01, 0x01, 0x0a, 57, 0xff],
    );
    assert_eq!(result.unwrap(), true);

    let result = button_handler.unregister();
    assert_eq!(result.unwrap(), true);

    let result = sensor_handler.unregister();
    assert_eq!(result.unwrap(), true);
}
