use clap::{App, Arg};
use core_cube::win10::*;
use ctrlc;
use env_logger;
use lazy_static::lazy_static;
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{cmp, thread, time};

const MOTOR_FW: u8 = 0x01;
const MOTOR_RV: u8 = 0x02;

const CIRCLE_TERM_MS: u64 = 7500;

static MAT_ENABLE: OnceCell<bool> = OnceCell::new();
static MAT_OFFSET_X: OnceCell<usize> = OnceCell::new();
static MAT_OFFSET_Y: OnceCell<usize> = OnceCell::new();

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
    SwingR,
    SwingL,
    Step2,
    Step4,
    Step8,
    StepRL2,
    RollingR,
    RollingL,
    GetReady1,
    GetReady1Short,
    GetReady2,
    ByeBye,
    Circle1,
    Circle2,
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
    term_ms: Option<u64>, // None: default time
    data: Option<Vec<u8>>,
}

impl Default for CubeControl {
    fn default() -> Self {
        Self {
            command: CubeCommand::Nothing,
            term_ms: None,
            data: None,
        }
    }
}

struct CubeInfo {
    id: usize,
    ble: CoreCubeBLE,
    action: CubeAction,
    step_count: usize,
    action_term: time::Duration,
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
        CubeAction::SwingR,
        CubeAction::SwingL,
        CubeAction::Step2,
        CubeAction::Step4,
        CubeAction::StepRL2,
        CubeAction::RollingR,
        CubeAction::RollingL,
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

fn get_cube_control_data(cube: &CubeInfo, max_duration: u64) -> Option<CubeControl> {
    let motor_duration = ((max_duration * 4) / 5) as u8;
    let control = match cube.action {
        CubeAction::GetReady1 | CubeAction::GetReady1Short => {
            if !MAT_ENABLE.get().unwrap() {
                return None;
            }
            let x = match cube.id % 4 {
                0 => MAT_OFFSET_X.get().unwrap() + 200,
                1 => MAT_OFFSET_X.get().unwrap() + 300,
                2 => MAT_OFFSET_X.get().unwrap() + 150,
                3 => MAT_OFFSET_X.get().unwrap() + 350,
                _ => MAT_OFFSET_X.get().unwrap() + 250,
            };
            let x_upper = (x / 256) as u8;
            let x_lower = (x % 256) as u8;

            let y_offset = (cube.id / 2) * 50;
            let y = match cube.action {
                CubeAction::GetReady1 => MAT_OFFSET_Y.get().unwrap() + 200 + y_offset,
                CubeAction::GetReady1Short => MAT_OFFSET_Y.get().unwrap() + 220 + y_offset,
                _ => MAT_OFFSET_Y.get().unwrap() + 230 + y_offset,
            };
            let y_upper = (y / 256) as u8;
            let y_lower = (y % 256) as u8;

            let speed = match cube.action {
                CubeAction::GetReady1 => 80,
                CubeAction::GetReady1Short => 30,
                _ => 30,
            };
            let term_ms = match cube.action {
                CubeAction::GetReady1 => 4000,
                CubeAction::GetReady1Short => 400,
                _ => 1500,
            };
            let moving_type = match cube.action {
                CubeAction::GetReady1 => 3,
                CubeAction::GetReady1Short => 0,
                _ => 0,
            };
            match cube.step_count {
                0 => CubeControl {
                    command: CubeCommand::MoveTo,
                    term_ms: Some(term_ms),
                    data: Some(vec![
                        4,
                        0,
                        speed,
                        moving_type,
                        x_lower,
                        x_upper,
                        y_lower,
                        y_upper,
                        90,
                        0,
                    ]),
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::Circle1 => {
            let speed: u8 = 50;
            let mut speed_r = speed;
            let mut speed_l = speed;
            if cube.id == 0 {
                speed_l -= 8;
            } else {
                speed_l = 0;
                speed_r = 0;
            }

            match cube.step_count {
                0 => CubeControl {
                    command: CubeCommand::Move,
                    term_ms: Some(CIRCLE_TERM_MS),
                    data: Some(vec![MOTOR_FW, speed_l, MOTOR_FW, speed_r, 0]),
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::Circle2 => {
            let speed: u8 = 50;
            let mut speed_r = speed;
            let mut speed_l = speed;
            if cube.id == 1 {
                speed_r -= 8;
            } else {
                speed_l = 0;
                speed_r = 0;
            }
            match cube.step_count {
                0 => CubeControl {
                    command: CubeCommand::Move,
                    term_ms: Some(CIRCLE_TERM_MS - 100),
                    data: Some(vec![MOTOR_FW, speed_l, MOTOR_FW, speed_r, 0]),
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::ByeBye => {
            if !MAT_ENABLE.get().unwrap() {
                return None;
            }
            println!("{}", cube.step_count);
            let speed: u8 = 30;
            let state = match cube.step_count {
                0..=19 => cube.step_count % 2,
                _ => cube.step_count - 19,
            };

            let x = match cube.id % 2 {
                0 => MAT_OFFSET_X.get().unwrap() + 80,
                1 => MAT_OFFSET_X.get().unwrap() + 420,
                _ => MAT_OFFSET_X.get().unwrap() + 250,
            };
            let x_upper = (x / 256) as u8;
            let x_lower = (x % 256) as u8;

            let y = match cube.id % 4 {
                0 | 1 => MAT_OFFSET_Y.get().unwrap() + 70,
                2 | 3 => MAT_OFFSET_Y.get().unwrap() + 430,
                _ => MAT_OFFSET_Y.get().unwrap() + 200,
            };
            let y_upper = (y / 256) as u8;
            let y_lower = (y % 256) as u8;

            let d = 270;
            let d_upper = (d / 256) as u8;
            let d_lower = (d % 256) as u8;

            match state {
                0 => CubeControl {
                    command: CubeCommand::Move,
                    term_ms: Some(100),
                    data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, 0]),
                },
                1 => CubeControl {
                    command: CubeCommand::Move,
                    term_ms: Some(100),
                    data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed, 0]),
                },
                2 => CubeControl {
                    command: CubeCommand::Move,
                    term_ms: Some(30),
                    data: Some(vec![MOTOR_RV, 0, MOTOR_RV, 0, 0]),
                },
                3 => CubeControl {
                    command: CubeCommand::MoveTo,
                    term_ms: Some(5000),
                    data: Some(vec![
                        4, 0, 80, 3, x_lower, x_upper, y_lower, y_upper, d_lower, d_upper,
                    ]),
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::GetReady2 => {
            let speed: u8 = 30;
            match cube.step_count {
                0 => CubeControl {
                    command: CubeCommand::Move,
                    term_ms: Some(3000),
                    data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, 0]),
                },
                1 => CubeControl {
                    command: CubeCommand::Move,
                    // Turn Left
                    data: Some(vec![MOTOR_RV, 15, MOTOR_FW, 15, 55]),
                    ..CubeControl::default()
                },
                2 => CubeControl {
                    command: CubeCommand::Nothing,
                    term_ms: Some(2000),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::SwingR => {
            let speed: u8 = 20;
            match cube.step_count {
                0 | 3 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_RV, speed, motor_duration]),
                    ..CubeControl::default()
                },
                1 | 2 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_FW, speed, motor_duration]),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::SwingL => {
            let speed: u8 = 20;
            match cube.step_count {
                0 | 3 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_FW, speed, motor_duration]),
                    ..CubeControl::default()
                },
                1 | 2 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_RV, speed, motor_duration]),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::Step2 => {
            let speed: u8 = 20;
            match cube.step_count {
                0 | 3 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, motor_duration]),
                    ..CubeControl::default()
                },
                1 | 2 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed, motor_duration]),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::Step4 => {
            let speed: u8 = 20;
            match cube.step_count {
                0 | 1 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, motor_duration]),
                    ..CubeControl::default()
                },
                2 | 3 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed, motor_duration]),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::Step8 => {
            let speed: u8 = 20;
            match cube.step_count {
                0 | 1 | 2 | 3 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed, motor_duration]),
                    ..CubeControl::default()
                },
                4 | 5 | 6 | 7 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed, motor_duration]),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::StepRL2 => {
            let speed: u8 = 20;
            match cube.step_count {
                0 | 3 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_FW, speed / 2, motor_duration]),
                    ..CubeControl::default()
                },
                1 | 2 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed / 2, MOTOR_FW, speed, motor_duration]),
                    ..CubeControl::default()
                },
                4 | 7 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed / 2, MOTOR_RV, speed, motor_duration]),
                    ..CubeControl::default()
                },
                5 | 6 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_RV, speed / 2, motor_duration]),
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::RollingR => {
            let speed: u8 = 37;
            let full_time = cmp::min(255, max_duration * 4) as u8;
            match cube.step_count {
                0 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_FW, speed, MOTOR_RV, speed, full_time]),
                    ..CubeControl::default()
                },
                1 | 2 | 3 => CubeControl {
                    command: CubeCommand::Nothing,
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        CubeAction::RollingL => {
            let speed: u8 = 37;
            let full_time = cmp::min(255, max_duration * 4) as u8;
            match cube.step_count {
                0 => CubeControl {
                    command: CubeCommand::Move,
                    data: Some(vec![MOTOR_RV, speed, MOTOR_FW, speed, full_time]),
                    ..CubeControl::default()
                },
                1 | 2 | 3 => CubeControl {
                    command: CubeCommand::Nothing,
                    ..CubeControl::default()
                },
                _ => CubeControl {
                    command: CubeCommand::End,
                    ..CubeControl::default()
                },
            }
        }
        _ => CubeControl {
            ..CubeControl::default()
        },
    };

    match control.command {
        CubeCommand::End => None,
        _ => Some(control),
    }
}

fn send_commnand_to_cube(cube: &mut CubeInfo, control: &CubeControl, default_action_term_ms: u64) {
    println!("{:?}", control.command);
    match control.command {
        CubeCommand::Move => {
            if let Some(data) = &control.data {
                let ble_data: Vec<u8> = vec![
                    0x02, 0x01, data[0], data[1], 0x02, data[2], data[3], data[4],
                ];
                let result = cube.ble.write(CoreCubeUuidName::MotorCtrl, &ble_data);
                assert_eq!(result.unwrap(), true);
            }
            cube.step_count += 1;
            if let Some(action_term_ms) = control.term_ms {
                cube.action_term = time::Duration::from_millis(action_term_ms);
                println!("action_term_ms");
            } else {
                cube.action_term = time::Duration::from_millis(default_action_term_ms);
                println!("default_action_term_ms");
            }
        }
        CubeCommand::MoveTo => {
            if !MAT_ENABLE.get().unwrap() {
                return;
            }
            println!("MoveTo");
            if let Some(data) = &control.data {
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
                    0x00,
                    x_l,
                    x_u,
                    y_l,
                    y_u,
                    degree_l,
                    degree_u,
                ];
                println!("{:?}", ble_data);
                let result = cube.ble.write(CoreCubeUuidName::MotorCtrl, &ble_data);
                assert_eq!(result.unwrap(), true);
            }
            cube.step_count += 1;
            if let Some(action_term_ms) = control.term_ms {
                cube.action_term = time::Duration::from_millis(action_term_ms);
            } else {
                cube.action_term = time::Duration::from_millis(default_action_term_ms);
            }
        }
        CubeCommand::MoveToMulti => {
            cube.step_count = 0;
            if let Some(action_term_ms) = control.term_ms {
                cube.action_term = time::Duration::from_millis(action_term_ms);
            } else {
                cube.action_term = time::Duration::from_millis(default_action_term_ms);
            }
        }
        CubeCommand::End => {
            cube.step_count = 0;
            cube.action_term = time::Duration::from_millis(0);
        }
        CubeCommand::Nothing => {
            cube.step_count += 1;
            if let Some(action_term_ms) = control.term_ms {
                cube.action_term = time::Duration::from_millis(action_term_ms);
            } else {
                cube.action_term = time::Duration::from_millis(default_action_term_ms);
            }
        }
    }
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
            Arg::with_name("cube")
                .help("max cube number")
                .long("cube")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mat")
                .help("mat type")
                .long("mat")
                .takes_value(true)
                .possible_values(&["tc1", "tc2", "gesun", "none"])
                .default_value("none"),
        );

    // Parse arguments
    let matches = app.get_matches();

    let mut default_action_term_ms: u64 = 600;
    if let Some(tempo_str) = matches.value_of("tempo") {
        default_action_term_ms = match u64::from_str_radix(&tempo_str, 10) {
            Ok(tempo) => (1000 * 1000) / (tempo * 1000 / 60),
            Err(e) => {
                error!("{}", e);
                120
            }
        };
        println!(
            "tempo {} (default_action_term_ms:{}[ms])",
            tempo_str, default_action_term_ms
        );
    }

    let mut cube_max: usize = 1;
    if let Some(cube_str) = matches.value_of("cube") {
        cube_max = match usize::from_str_radix(&cube_str, 10) {
            Ok(cube_num) => cube_num,
            Err(e) => {
                error!("{}", e);
                1
            }
        };
        println!("using {} cubes", cube_max);
    }

    match matches.value_of("mat").unwrap() {
        "tc1" => {
            println!("Use toio collection mat (circle side)");
            MAT_ENABLE.set(true).unwrap();
            MAT_OFFSET_X.set(0).unwrap();
            MAT_OFFSET_Y.set(0).unwrap();
        }
        "tc2" => {
            println!("Use toio collection mat (checker side)");
            MAT_ENABLE.set(true).unwrap();
            MAT_OFFSET_X.set(500).unwrap();
            MAT_OFFSET_Y.set(0).unwrap();
        }
        "gesun" => {
            println!("Use gesundroiod mat");
            MAT_ENABLE.set(true).unwrap();
            MAT_OFFSET_X.set(1000).unwrap();
            MAT_OFFSET_Y.set(0).unwrap();
        }
        _ => {
            println!("Without mat mode");
            MAT_ENABLE.set(true).unwrap();
            MAT_OFFSET_X.set(0).unwrap();
            MAT_OFFSET_Y.set(0).unwrap();
        }
    };

    // connect
    let mut cube: Vec<CubeInfo> = Vec::with_capacity(cube_max);

    for i in 0..cube_max {
        println!("connect cube {}", i);
        let c = match connect_ref_id() {
            Ok(x) => x,
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            }
        };
        let info = CubeInfo {
            id: i,
            ble: c,
            action: CubeAction::GetReady1,
            step_count: 0,
            action_term: time::Duration::from_millis(0),
        };
        cube.push(info);
    }

    let led_color: Vec<Vec<u8>> = vec![
        vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x10],
        vec![0x03, 0x00, 0x01, 0x01, 0x10, 0x00, 0x00],
        vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x10, 0x00],
        vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x10, 0x10],
    ];
    for i in 0..cube_max {
        // LED on 
        let result = cube[i]
            .ble
            .write(CoreCubeUuidName::LightCtrl, &led_color[i % led_color.len()]);
        assert_eq!(result.unwrap(), true);

        // Set collision detection level: Level 10
        let result = cube[i]
            .ble
            .write(CoreCubeUuidName::Configuration, &vec![0x06, 0x00, 0x0a]);
        assert_eq!(result.unwrap(), true);

        // Set double-tap detection time: Level 2
        let result = cube[i]
            .ble
            .write(CoreCubeUuidName::Configuration, &vec![0x17, 0x00, 0x04]);
        assert_eq!(result.unwrap(), true);
    }

    // Register cube notify handlers
    let result = cube[0]
        .ble
        .register_norify(CoreCubeUuidName::ButtonInfo, Box::new(button_notify));
    let button_handler = result.unwrap();

    let result = cube[0].ble.register_norify(
        CoreCubeUuidName::SensorInfo,
        Box::new(sensor_information_notify_1),
    );
    let sensor_handler = result.unwrap();

    let result = cube[0]
        .ble
        .register_norify(CoreCubeUuidName::IdInfo, Box::new(id_information_notify));
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

    let cube_max_duration = cmp::min(255, (default_action_term_ms / 10) & 0xff);
    let cube_motor_duration = ((cube_max_duration * 4) / 5) as u8;
    println!(
        "cube_max_duration:{}, cube_motor_duration:{}",
        cube_max_duration, cube_motor_duration
    );

    let action_list = [
        [CubeAction::GetReady1, CubeAction::GetReady1],
        [CubeAction::Circle1, CubeAction::Circle1],
        [CubeAction::GetReady1Short, CubeAction::GetReady1Short],
        [CubeAction::Circle2, CubeAction::Circle2],
        [CubeAction::GetReady1Short, CubeAction::GetReady1Short],
        [CubeAction::SwingR, CubeAction::SwingL],
        [CubeAction::Step2, CubeAction::Step2],
        [CubeAction::RollingR, CubeAction::RollingR],
        [CubeAction::RollingL, CubeAction::RollingL],
        [CubeAction::GetReady1Short, CubeAction::GetReady1Short],
        [CubeAction::StepRL2, CubeAction::StepRL2],
        [CubeAction::Step8, CubeAction::Step8],
        [CubeAction::RollingL, CubeAction::RollingR],
        [CubeAction::RollingR, CubeAction::RollingL],
        [CubeAction::GetReady1Short, CubeAction::GetReady1Short],
        [CubeAction::ByeBye, CubeAction::ByeBye],
    ];

    let mut action_count = 0;
    while running.load(Ordering::SeqCst) {
        let mut next_action: bool = false;
        for i in 0..cube_max {
            loop {
                let motor_control_data: Option<CubeControl> =
                    get_cube_control_data(&cube[i], cube_max_duration);
                if let Some(control) = motor_control_data {
                    send_commnand_to_cube(&mut cube[i], &control, default_action_term_ms);
                    break;
                } else {
                    next_action = true;
                    break;
                }
            }
        }

        if next_action {
            action_count += 1;
            if action_count >= action_list.len() {
                break;
            }
            for i in 0..cube_max {
                cube[i].step_count = 0;
                cube[i].action = action_list[action_count][i % 2];
            }
            println!(
                "cube 0 next {:?}, cube 1 next {:?}",
                cube[0].action, cube[1].action
            );
        } else {
            let mut max_action_term = time::Duration::from_millis(0);
            for cube_info in &cube {
                println!(
                    "  cube {} action_term {:?}",
                    cube_info.id, cube_info.action_term
                );
                max_action_term = cmp::max(max_action_term, cube_info.action_term);
            }
            println!("max_action_term {:?}", max_action_term);

            if max_action_term > time::Duration::from_millis(0) {
                thread::sleep(max_action_term);
            }
        }
    }
    // --------------------------------------------------------------------------------

    // LED off
    for i in 0..cube_max {
        let result = cube[i].ble.write(
            CoreCubeUuidName::LightCtrl,
            &vec![0x03, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00],
        );
        assert_eq!(result.unwrap(), true);
    }

    // beep
    let result = cube[0].ble.write(
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
