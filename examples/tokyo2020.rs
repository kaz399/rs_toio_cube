use clap::{App, Arg};
use core_cube::win10::*;
use ctrlc;
use env_logger;
use lazy_static::lazy_static;
use log::{debug, error, info};
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{cmp, thread, time};

const MOTOR_FW: u8 = 0x01;
const MOTOR_RV: u8 = 0x02;

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

#[derive(Debug, Copy, Clone, PartialEq)]
enum CubeAction {
    SwingL,
    SwingR,
    Step2,
    Step4,
    StepLR2,
    RollingL,
    RollingR,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum CubeCommand {
    Move,
    MoveTo,
    MoveToMulti,
    Nothing,
    End,
}

#[derive(Debug)]
struct CubeControl {
    command: CubeCommand,
    data: Option<Vec<u8>>,
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

//    [ Swing, Step4, Step2, Rolling_type1, Rollong_type2]
// choose next cube action
fn get_next_cube_action() -> CubeAction {
    let mut rng = rand::thread_rng();
    let actions = [
        CubeAction::SwingL,
        CubeAction::SwingR,
        CubeAction::Step2,
        CubeAction::Step4,
        CubeAction::StepLR2,
        CubeAction::RollingL,
        CubeAction::RollingR,
    ];
    let weight = [4, 4, 4, 3, 5, 5, 5];

    let mut random_max = 0;
    for value in weight {
        random_max += value;
    }

    let random_number = rng.gen_range(0..random_max);
    let mut next_action_number = 0;

    let mut sum = 0;
    for (num, value) in weight.iter().enumerate() {
        sum += value;
        if random_number < sum {
            next_action_number = num;
            break;
        }
    }

    actions[next_action_number]
}

fn main() {
    env_logger::init();

    // Set command line options
    let app = App::new("example")
        .version("0.0.1")
        .arg(
            Arg::with_name("tempo")
                .help("tempo")
                .long("tempo")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cubes")
                .help("total cubes")
                .long("cubes")
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
            Ok(tempo) => (1000 * 1000) / (tempo * 1000 / 60),
            Err(e) => {
                error!("{}", e);
                120
            }
        };
        println!("tempo {} (interval:{}[ms])", tempo_str, interval);
    }

    let mut cube_max: usize = 1;
    if let Some(cubes_str) = matches.value_of("cubes") {
        cube_max = match usize::from_str_radix(&cubes_str, 10) {
            Ok(cube_num) => cube_num,
            Err(e) => {
                error!("{}", e);
                1
            }
        };
        println!("using {} cubes", cube_max);
    }

    // connect
    let mut cube: Vec<CoreCubeBLE> = Vec::with_capacity(cube_max);

    for i in 0..cube_max {
        println!("connect cube {}", i);
        let c = match connect_ref_id() {
            Ok(x) => x,
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            }
        };
        cube.push(c);
    }

    // LED on (blue)
    let result = cube[0].write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x10],
    );
    assert_eq!(result.unwrap(), true);

    // cube2: LED on (red)
    let result = cube[1].write(
        CoreCubeUuidName::LightCtrl,
        &vec![0x03, 0x00, 0x01, 0x01, 0x10, 0x00, 0x00],
    );
    assert_eq!(result.unwrap(), true);

    // cube3: LED on (green)
    if cube_max >= 3 {
        let result = cube[2].write(
            CoreCubeUuidName::LightCtrl,
            &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x10, 0x00],
        );
        assert_eq!(result.unwrap(), true);
    }

    // Set collision detection level: Level 10
    let result = cube[0].write(CoreCubeUuidName::Configuration, &vec![0x06, 0x00, 0x0a]);
    assert_eq!(result.unwrap(), true);

    // Set double-tap detection time: Level 2
    let result = cube[0].write(CoreCubeUuidName::Configuration, &vec![0x17, 0x00, 0x04]);
    assert_eq!(result.unwrap(), true);

    // Register cube notify handlers
    let result = cube[0].register_norify(CoreCubeUuidName::ButtonInfo, Box::new(button_notify));
    let button_handler = result.unwrap();

    let result = cube[0].register_norify(
        CoreCubeUuidName::SensorInfo,
        Box::new(sensor_information_notify_1),
    );
    let sensor_handler = result.unwrap();

    let result = cube[0].register_norify(CoreCubeUuidName::IdInfo, Box::new(id_information_notify));
    let id_handler = result.unwrap();

    // cube2: Set collision detection level: Level 10
    let result = cube[1].write(CoreCubeUuidName::Configuration, &vec![0x06, 0x00, 0x0a]);
    assert_eq!(result.unwrap(), true);

    // cube2: Set double-tap detection time: Level 2
    let result = cube[1].write(CoreCubeUuidName::Configuration, &vec![0x17, 0x00, 0x04]);
    assert_eq!(result.unwrap(), true);

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
    let mut step_count: usize = 0;

    let max_duration = cmp::min(255, (interval / 10) & 0xff);
    let motor_duration = ((max_duration * 4) / 5) as u8;
    println!(
        "max_duration:{}, motor_duration:{}",
        max_duration, motor_duration
    );
    let mut cube_action = CubeAction::Step2;

    while running.load(Ordering::SeqCst) {
        let motor_control_data: Option<CubeControl> = match cube_action {
            CubeAction::SwingL => {
                let speed: u8 = 20;
                let motor_ctrl: CubeControl = match step_count {
                    0 | 3 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed, MOTOR_RV, speed, motor_duration]),
                    },
                    1 | 2 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed, MOTOR_FW, speed, motor_duration]),
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
            CubeAction::SwingR => {
                let speed: u8 = 20;
                let motor_ctrl: CubeControl = match step_count {
                    0 | 3 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed, MOTOR_FW, speed, motor_duration]),
                    },
                    1 | 2 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed, MOTOR_RV, speed, motor_duration]),
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
            CubeAction::Step2 => {
                let speed: u8 = 20;
                let motor_ctrl: CubeControl = match step_count {
                    0 | 3 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, motor_duration]),
                    },
                    1 | 2 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed, motor_duration]),
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
            CubeAction::Step4 => {
                let speed: u8 = 20;
                let motor_ctrl: CubeControl = match step_count {
                    0 | 1 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, motor_duration]),
                    },
                    2 | 3 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed, motor_duration]),
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
            CubeAction::StepLR2 => {
                let speed: u8 = 20;
                let motor_ctrl: CubeControl = match step_count {
                    0 | 3 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed / 2, motor_duration]),
                    },
                    1 | 2 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed / 2, MOTOR_FW, speed, motor_duration]),
                    },
                    4 | 7 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed / 2, MOTOR_RV, speed, motor_duration]),
                    },
                    5 | 6 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed / 2, motor_duration]),
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
            CubeAction::RollingL => {
                let speed: u8 = 37;
                let full_time = cmp::min(255, max_duration * 4) as u8;
                let motor_ctrl: CubeControl = match step_count {
                    0 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_FW, speed, MOTOR_RV, speed, full_time]),
                    },
                    1 | 2 | 3 => CubeControl {
                        command: CubeCommand::Nothing,
                        data: None,
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
            CubeAction::RollingR => {
                let speed: u8 = 37;
                let full_time = cmp::min(255, max_duration * 4) as u8;
                let motor_ctrl: CubeControl = match step_count {
                    0 => CubeControl {
                        command: CubeCommand::Move,
                        data: Some(vec![MOTOR_RV, speed, MOTOR_FW, speed, full_time]),
                    },
                    1 | 2 | 3 => CubeControl {
                        command: CubeCommand::Nothing,
                        data: None,
                    },
                    _ => CubeControl {
                        command: CubeCommand::End,
                        data: None,
                    },
                };
                Some(motor_ctrl)
            }
        };

        if let Some(control) = motor_control_data {
            match control.command {
                CubeCommand::Move => {
                    if let Some(data) = control.data {
                        let ble_data: Vec<u8> = vec![
                            0x02, 0x01, data[0], data[1], 0x02, data[2], data[3], data[4],
                        ];
                        for i in 0..cube_max {
                            let result = cube[i].write(CoreCubeUuidName::MotorCtrl, &ble_data);
                            assert_eq!(result.unwrap(), true);
                        }
                    }
                    step_count += 1;
                    thread::sleep(tick);
                }
                CubeCommand::MoveTo => {
                    if let Some(data) = control.data {
                        let timeout = data[0];
                        let moving_type = data[1];
                        let max_speed = data[2];
                        let acceleration = data[3];
                        let x_l = data[4];
                        let x_u = data[5];
                        let y_l = data[6];
                        let y_u = data[7];
                        let degree_l = data[8];
                        let degree_u = data[9];
                        let ble_data: Vec<u8> = vec![
                            0x03,
                            0x00,
                            timeout,
                            moving_type,
                            max_speed,
                            acceleration,
                            x_l,
                            x_u,
                            y_l,
                            y_u,
                            degree_l,
                            degree_u,
                        ];
                        for i in 0..cube_max {
                            let result = cube[i].write(CoreCubeUuidName::MotorCtrl, &ble_data);
                            assert_eq!(result.unwrap(), true);
                        }
                    }
                }
                CubeCommand::MoveToMulti => {}
                CubeCommand::End => {
                    step_count = 0;
                    cube_action = get_next_cube_action();
                    println!("next action:{:?}", cube_action);
                }
                CubeCommand::Nothing => {
                    step_count += 1;
                    thread::sleep(tick);
                }
            }
        }
    }
    // --------------------------------------------------------------------------------

    // LED off
    for i in 0..cube_max {
        let result = cube[i].write(
            CoreCubeUuidName::LightCtrl,
            &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00],
        );
        assert_eq!(result.unwrap(), true);
    }

    // beep
    let result = cube[0].write(
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
