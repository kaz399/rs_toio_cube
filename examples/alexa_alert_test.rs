use clap::{App, Arg};
use core_cube::win10::*;
use ctrlc;
use env_logger;
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{cmp, thread, time};

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
    RightSideUp = 5,
    LeftSideUp = 6,
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
fn button_notify(data: Vec<u8>) {
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
}

fn get_sensor_info(data: Vec<u8>) -> SensorInfo {
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
            0x05 => PostureStatus::RightSideUp,
            0x06 => PostureStatus::LeftSideUp,
            _ => PostureStatus::Unknown,
        },
        shaking: data[5] as usize,
    }
}

// cube1 Sensor Notify Handler
fn sensor_information_notify_1(data: Vec<u8>) {
    debug!("sensor(cube1) information status changed {:?}", data);
    {
        let mut sensor = SENSOR_1.lock().unwrap();
        (*sensor).push(get_sensor_info(data));
    }
}

// ID Information Notify Handler
fn id_information_notify(data: Vec<u8>) {
    info!("id information status changed {:?}", data);
}

// Connect by ref_id (paired cube)
fn connect_ref_id() -> std::result::Result<CoreCubeBLE, String> {
    loop {
        let mut cube = CoreCubeBLE::new("Cube1".to_string());
        println!("search registered cubes");
        let dev_list = get_ble_devices().unwrap();
        if dev_list.len() == 0 {
            return Err("failed to connect".to_string());
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

fn main() {
    env_logger::init();


    // Set command line options
    let app = App::new("example")
        .version("0.0.1")
        .arg(Arg::with_name("aaa").help("option a").long("aaa"))
        .arg(Arg::with_name("bbb").help("option b").long("bps"))
        .arg(
            Arg::with_name("tempo")
            .help("tempo")
            .long("tempo")
            .takes_value(true))
        .arg(
            Arg::with_name("address")
            .help("BLE address")
            .long("address")
            .takes_value(true),
        );

    // Parse arguments
    let matches = app.get_matches();
    if matches.is_present("aaa") {
        println!("option aaa");
    } else if matches.is_present("bbb") {
        println!("option bbb");
    } else {
        println!("no option");
    }

    let mut interval: u64 = 600;
    if let Some(tempo_str) = matches.value_of("tempo") {
        interval = match u64::from_str_radix(&tempo_str, 10) {
            Ok(tempo) => (1000 * 1000) /  (tempo* 1000 / 60),
            Err(e) => {
                error!("{}", e);
                120
            }
        };
        println!("tempo {} (interval:{}[ms])", tempo_str, interval);
    }

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


    // LED on (green)
    let result = cube.write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x10, 0x00],
    );
    assert_eq!(result.unwrap(), true);

    // Set collision detection level: Level 10
    let result = cube.write(CoreCubeUuidName::Configuration, &vec![0x06, 0x00, 0x0a]);
    assert_eq!(result.unwrap(), true);

    // Set double-tap detection time: Level 2
    let result = cube.write(CoreCubeUuidName::Configuration, &vec![0x17, 0x00, 0x04]);
    assert_eq!(result.unwrap(), true);

    // Register cube notify handlers
    let result = cube.register_notify(CoreCubeUuidName::ButtonInfo, Box::new(button_notify));
    let button_handler = result.unwrap();

    let result = cube.register_notify(CoreCubeUuidName::SensorInfo, Box::new(sensor_information_notify_1));
    let sensor_handler = result.unwrap();

    let result = cube.register_notify(CoreCubeUuidName::IdInfo, Box::new(id_information_notify));
    let id_handler = result.unwrap();

    // Register Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // MAIN LOOP
    // --------------------------------------------------------------------------------


    let tick = time::Duration::from_millis(interval);
    let mut loop_count: usize = 0;

    let max_duration = cmp::min(255, (interval / 10) & 0xff);
    let motor_duration = ((max_duration * 2) / 3) as u8;
    println!("max_duration:{}, motor_duration:{}", max_duration, motor_duration);

    const MOTOR_FW: u8 = 0x01;
    const MOTOR_RV: u8 = 0x02;


    let action = vec![
        vec![MOTOR_FW, 100, MOTOR_RV, 100, 40],
        vec![MOTOR_RV,  10, MOTOR_FW,  10, 40],
        vec![MOTOR_FW,  10, MOTOR_FW,  10, 40],
        vec![MOTOR_RV,  10, MOTOR_RV,  10, 40],
        vec![MOTOR_FW,  10, MOTOR_FW,  10, 40],
        vec![MOTOR_RV,  10, MOTOR_RV,  10, 40],
        vec![MOTOR_FW,  10, MOTOR_FW,  10, 40],
        vec![MOTOR_RV,  10, MOTOR_RV,  10, 40],

        vec![MOTOR_RV,  70, MOTOR_FW,  70, 40],
    ];

    while running.load(Ordering::SeqCst) {
        let action_step = loop_count % action.len();
        if action_step == 0 {
            thread::sleep(tick * 2);
        }
        println!("count {}", action_step);

        let control = &action[action_step];
        let motor_bytestream = vec![0x02, 0x01, control[0], control[1], 0x02, control[2], control[3], control[4]];


        let result = cube.write(
            CoreCubeUuidName::MotorCtrl,
            &motor_bytestream);
        assert_eq!(result.unwrap(), true);

        thread::sleep(tick);
        loop_count += 1;
    }
    // --------------------------------------------------------------------------------

    // LED off
    let result = cube.write(
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
}
