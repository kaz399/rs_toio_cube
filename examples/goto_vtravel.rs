use clap::{App, Arg};
use core_cube::win10::*;
use ctrlc;
use enigo::*;
use env_logger;
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{thread, time};

const KEYEVENT_THRESH_LR: usize = 10;
const KEYEVENT_THRESH_UP: usize = 10;


#[derive(Debug, Copy, Clone, PartialEq)]
enum ButtonStatus {
    Unknown,
    Press,
    Release,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SlopeStatus {
    Unknown,
    Aslant,
    Horizontal,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum CollisionStatus {
    Unknown,
    NotDetect,
    Detect,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum DoubleTapStatus {
    Unknown,
    NotDetect,
    Detect,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum PostureStatus {
    Unknown = 0,
    Normal = 1,
    Reverse = 2,
    Downward = 3,
    Upward = 4,
    RigitSideUp = 5,
    LeftSideUp = 6,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum KeyTableName {
    Page = 0,
    LR = 1,
    UD = 2,
}

#[derive(Debug, Copy, Clone)]
struct ButtonInfo {
    time: time::Instant,
    button: ButtonStatus,
}

impl Default for ButtonInfo {
    fn default() -> Self {
        Self {
            time: time::Instant::now(),
            button: ButtonStatus::Unknown,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct SensorInfo {
    time: time::Instant,
    slope: SlopeStatus,
    collision: CollisionStatus,
    double_tap: DoubleTapStatus,
    posture: PostureStatus,
    shaking: usize,
}

impl Default for SensorInfo {
    fn default() -> Self {
        Self {
            time: time::Instant::now(),
            slope: SlopeStatus::Unknown,
            collision: CollisionStatus::Unknown,
            double_tap: DoubleTapStatus::Unknown,
            posture: PostureStatus::Unknown,
            shaking: 0,
        }
    }
}

lazy_static! {
    static ref BUTTON: Mutex<Vec<ButtonInfo>> = Mutex::new(Vec::new());
    static ref SENSOR_1: Mutex<Vec<SensorInfo>> = Mutex::new(Vec::new());
    static ref SENSOR_2: Mutex<Vec<SensorInfo>> = Mutex::new(Vec::new());
}

// Button Notify Handler
fn button_notify(_sender: &CoreCubeNotifySender, arg: &CoreCubeNotifyArgs) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("button status changed {:?}", data);
    let button_info = ButtonInfo {
        time: time::Instant::now(),
        button: match data[1] {
            0x00 => ButtonStatus::Release,
            0x80 => ButtonStatus::Press,
            _ => ButtonStatus::Unknown,
        },
    };
    {
        let mut button = BUTTON.lock().unwrap();
        (*button).push(button_info);
    }
    Ok(())
}

fn get_sensor_info(data: Vec::<u8>) -> SensorInfo {
    SensorInfo {
        time: time::Instant::now(),
        slope: match data[1] {
            0x00 => SlopeStatus::Aslant,
            0x01 => SlopeStatus::Horizontal,
            _ => SlopeStatus::Unknown,
        },
        collision: match data[2] {
            0x00 => CollisionStatus::NotDetect,
            0x01 => CollisionStatus::Detect,
            _ => CollisionStatus::Unknown,
        },
        double_tap: match data[3] {
            0x00 => DoubleTapStatus::NotDetect,
            0x01 => DoubleTapStatus::Detect,
            _ => DoubleTapStatus::Unknown,
        },
        posture: match data[4] {
            0x01 => PostureStatus::Normal,
            0x02 => PostureStatus::Reverse,
            0x03 => PostureStatus::Downward,
            0x04 => PostureStatus::Upward,
            0x05 => PostureStatus::RigitSideUp,
            0x06 => PostureStatus::LeftSideUp,
            _ => PostureStatus::Unknown,
        },
        shaking: data[5] as usize,
    }
}

// cube1 Sensor Notify Handler
fn sensor_information_notify_1(
    _sender: &CoreCubeNotifySender,
    arg: &CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("sensor(cube1) information status changed {:?}", data);
    {
        let mut sensor = SENSOR_1.lock().unwrap();
        (*sensor).push(get_sensor_info(data));
    }
    Ok(())
}

// cube2 Sensor Notify Handler
fn sensor_information_notify_2(
    _sender: &CoreCubeNotifySender,
    arg: &CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("sensor(cube2) information status changed {:?}", data);
    {
        let mut sensor = SENSOR_2.lock().unwrap();
        (*sensor).push(get_sensor_info(data));
    }
    Ok(())
}

// ID Information Notify Handler
fn id_information_notify(
    _sender: &CoreCubeNotifySender,
    arg: &CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    info!("id information status changed {:?}", data);
    Ok(())
}

// Connect by ref_id (paired cube)
fn connect_ref_id() -> std::result::Result<CoreCubeBLE, String> {
    loop {
        let mut cube = CoreCubeBLE::new("Cube1".to_string());
        println!("search registered cubes");
        let dev_list = get_ble_devices().unwrap();
        if dev_list.len() == 0 {
            return Err("failed to conenct".to_string());
        }

        'search_next: for device_info in &dev_list {
            info!("Searching cube: {:?}", device_info);
            'connect_again: loop {
                let result = cube.connect_ref_id(device_info);
                match result.unwrap() {
                    true => {
                        let result = cube.read(CoreCubeUuidName::BatteryInfo);
                        match result {
                            Ok(v) => {
                                println!("success to connect");
                                println!("battery level {}%", v[0]);
                                if v[0] == 0 {
                                    error!("suspicious connection.. try to reconnect");
                                    continue 'connect_again;
                                }
                                return Ok(cube);
                            }
                            Err(_) => continue 'search_next,
                        }
                    }
                    false => {
                        info!("search next cube");
                        continue 'search_next;
                    }
                }
            }
        }
    }
}

// Connect by address
fn connect(address: u64) -> std::result::Result<CoreCubeBLE, String> {
    loop {
        let mut cube = CoreCubeBLE::new("Cube1".to_string());
        println!("search registered cubes");
        'connect_again: loop {
            let result = cube.connect(address);
            match result.unwrap() {
                true => {
                    let result = cube.read(CoreCubeUuidName::BatteryInfo);
                    match result {
                        Ok(v) => {
                            println!("success to connect");
                            println!("battery level {}%", v[0]);
                            if v[0] == 0 {
                                error!("suspicious connection.. try to reconnect");
                                continue 'connect_again;
                            }
                            return Ok(cube);
                        }
                        Err(_) => continue 'connect_again,
                    }
                }
                false => {
                    info!("search next cube");
                    continue 'connect_again;
                }
            }
        }
    }
}

fn get_sensor_info_list_1() -> Vec<SensorInfo> {
    let mut sensor = SENSOR_1.lock().unwrap();
    let sensor_info_list = (*sensor).clone();
    (*sensor).clear();
    if !sensor_info_list.is_empty() {
        debug!("cube1:sensor {:?}", sensor_info_list);
    }

    sensor_info_list
}

fn get_sensor_info_list_2() -> Vec<SensorInfo> {
    let mut sensor = SENSOR_2.lock().unwrap();
    let sensor_info_list = (*sensor).clone();
    (*sensor).clear();
    if !sensor_info_list.is_empty() {
        debug!("cube2:sensor {:?}", sensor_info_list);
    }

    sensor_info_list
}

fn main() {
    env_logger::init();

    let key_table: KeyTableName;

    // Set command line options
    let app = App::new("cubekey")
        .version("0.0.1")
        .arg(Arg::with_name("lr").help("LR arrow key mode").long("lr"))
        .arg(Arg::with_name("ud").help("UD arrow key mode").long("ud"))
        .arg(
            Arg::with_name("address")
                .help("BLE address")
                .long("address")
                .takes_value(true),
        );

    // Parse arguments
    let matches = app.get_matches();
    if matches.is_present("lr") {
        println!("LR arrow key mode");
        key_table = KeyTableName::LR;
    } else if matches.is_present("ud") {
        println!("UD arrow key mode");
        key_table = KeyTableName::UD;
    } else {
        println!("Page key mode");
        key_table = KeyTableName::Page;
    }
    debug!("key table {:?}", key_table);

    // connect
    let cube: CoreCubeBLE;
    if let Some(adrs_str) = matches.value_of("address") {
        let mut adrs = adrs_str.to_string();
        adrs.retain(|c| c != ':');
        adrs.retain(|c| c != '-');
        match u64::from_str_radix(&adrs, 16) {
            Ok(ble_adrs) => {
                cube = match connect(ble_adrs) {
                    Ok(x) => x,
                    Err(e) => {
                        error!("{}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            }
        }
    } else {
        cube = match connect_ref_id() {
            Ok(x) => x,
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            }
        }
    }

    let cube2: CoreCubeBLE;
    cube2 = match connect_ref_id() {
        Ok(x) => x,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    // LED on (green)
    let result = cube.write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x10, 0x00],
    );
    assert_eq!(result.unwrap(), true);

    // cube2: LED on (blue)
    let result = cube2.write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x10],
    );
    assert_eq!(result.unwrap(), true);

    // Set collision detection level: Level 10
    let result = cube.write(CoreCubeUuidName::Configuration, &vec![0x06, 0x00, 0x0a]);
    assert_eq!(result.unwrap(), true);

    // Set double-tap detection time: Level 2
    let result = cube.write(CoreCubeUuidName::Configuration, &vec![0x17, 0x00, 0x04]);
    assert_eq!(result.unwrap(), true);

    // Register cube notify handlers
    let result = cube.register_norify(CoreCubeUuidName::ButtonInfo, button_notify);
    let button_handler = result.unwrap();

    let result = cube.register_norify(CoreCubeUuidName::SensorInfo, sensor_information_notify_1);
    let sensor_handler = result.unwrap();

    let result = cube.register_norify(CoreCubeUuidName::IdInfo, id_information_notify);
    let id_handler = result.unwrap();


    // cube2: Set collision detection level: Level 10
    let result = cube2.write(CoreCubeUuidName::Configuration, &vec![0x06, 0x00, 0x0a]);
    assert_eq!(result.unwrap(), true);

    // cube2: Set double-tap detection time: Level 2
    let result = cube2.write(CoreCubeUuidName::Configuration, &vec![0x17, 0x00, 0x04]);
    assert_eq!(result.unwrap(), true);

    // cube2: Register cube notify handlers
    let result = cube2.register_norify(CoreCubeUuidName::SensorInfo, sensor_information_notify_2);
    let sensor_handler2 = result.unwrap();

    // Register Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // MAIN LOOP

    let mut engio = Enigo::new();
    let tick = time::Duration::from_millis(100);
    let mut integral_counter_cube1: usize = 0;
    let mut integral_counter_cube2: usize = 0;
    let mut integral_counter_inner_product: usize = 0;
    let mut key: Key = Key::Layout(' ');

    while running.load(Ordering::SeqCst) {
        if key != Key::Layout(' ') {
            info!("[KEYCODE( up )] {:?}", key);
            engio.key_up(key);
            key = Key::Layout(' ');
        }

        let shaking_cube1: usize;
        let shaking_cube2: usize;

        match get_sensor_info_list_1().pop() {
            Some(last) => shaking_cube1 = last.shaking,
            None => shaking_cube1 = 0,
        };

        match get_sensor_info_list_2().pop() {
            Some(last) => shaking_cube2 = last.shaking,
            None => shaking_cube2 = 0,
        };


        let inner_product = shaking_cube1 * shaking_cube2;
        if (inner_product) != 0 {
            integral_counter_cube1 = 0;
            integral_counter_cube2 = 0;
            integral_counter_inner_product += inner_product;
            if integral_counter_inner_product > KEYEVENT_THRESH_UP {
                key = Key::Layout('w');
            }
        } else {
            integral_counter_cube1 += shaking_cube1;
            integral_counter_cube2 += shaking_cube2;
            integral_counter_inner_product = 0;

            if integral_counter_cube1 > KEYEVENT_THRESH_LR {
                key = Key::Layout('a');
                integral_counter_cube1 = 0;
            } else {
                integral_counter_cube1 += shaking_cube1;
            }

            if integral_counter_cube2 > KEYEVENT_THRESH_LR {
                key = Key::Layout('d');
                integral_counter_cube2 = 0;
            } else {
                integral_counter_cube2 += shaking_cube2;
            }

        }

        if key != Key::Layout(' ') {
            info!("[KEYCODE(down)] {:?}", key);
            engio.key_down(key);
        }

        thread::sleep(tick);
    }

    // LED off
    let result = cube.write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00],
    );
    assert_eq!(result.unwrap(), true);

    // cube2: LED off
    let result = cube2.write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00],
    );
    assert_eq!(result.unwrap(), true);

    let result = cube.write(
        CoreCubeUuidName::SoundCtrl,
        &vec![0x03, 0x01, 0x01, 0x0a, 57, 0xff],
    );
    assert_eq!(result.unwrap(), true);

    let result = button_handler.unregister();
    assert_eq!(result.unwrap(), true);

    let result = sensor_handler.unregister();
    assert_eq!(result.unwrap(), true);

    let result = id_handler.unregister();
    assert_eq!(result.unwrap(), true);

    let result = sensor_handler2.unregister();
    assert_eq!(result.unwrap(), true);
}
