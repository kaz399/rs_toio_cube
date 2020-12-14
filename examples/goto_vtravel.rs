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
}

impl Default for SensorInfo {
    fn default() -> Self {
        Self {
            time: time::Instant::now(),
            slope: SlopeStatus::Unknown,
            collision: CollisionStatus::Unknown,
            double_tap: DoubleTapStatus::Unknown,
            posture: PostureStatus::Unknown,
        }
    }
}

lazy_static! {
    static ref BUTTON: Mutex<Vec<ButtonInfo>> = Mutex::new(Vec::new());
    static ref SENSOR: Mutex<Vec<SensorInfo>> = Mutex::new(Vec::new());
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

// Sensor Notify Hander
fn sensor_information_notify(
    _sender: &CoreCubeNotifySender,
    arg: &CoreCubeNotifyArgs,
) -> CoreCubeNotifyResult {
    let data = get_notify_data(arg);
    debug!("sensor information status changed {:?}", data);

    let now = time::Instant::now();
    {
        let mut sensor = SENSOR.lock().unwrap();
        (*sensor).push(SensorInfo {
            time: now,
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
        });
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

fn get_sensor_info_list() -> Vec<SensorInfo> {
    let mut sensor = SENSOR.lock().unwrap();
    let sensor_info_list = (*sensor).clone();
    (*sensor).clear();
    if !sensor_info_list.is_empty() {
        debug!("sensor {:?}", sensor_info_list);
    }

    sensor_info_list
}

fn get_button_info_list(duration: time::Duration) -> Vec<ButtonInfo> {
    let mut button = BUTTON.lock().unwrap();
    (*button).retain(|event| event.time.elapsed() <= duration);
    let button_info_list = (*button).clone();

    //debug!("list len (duration): {}", button_info_list.len());

    //if !button_info_list.is_empty() {
    //    for info in button_info_list.clone() {
    //        debug!("button {:?}", info.time.elapsed());
    //    }
    //}

    button_info_list
}

const CUBE_POSTURES: usize = 7;
const TABLE_TYPES: usize = 3;

static KEY_TABLE: [[Key; CUBE_POSTURES]; TABLE_TYPES] = [
    [
        Key::Escape,
        Key::PageUp,
        Key::PageDown,
        Key::PageUp,
        Key::PageUp,
        Key::PageUp,
        Key::PageUp,
    ],
    [
        Key::Escape,
        Key::LeftArrow,
        Key::RightArrow,
        Key::LeftArrow,
        Key::LeftArrow,
        Key::LeftArrow,
        Key::LeftArrow,
    ],
    [
        Key::Escape,
        Key::UpArrow,
        Key::DownArrow,
        Key::UpArrow,
        Key::UpArrow,
        Key::UpArrow,
        Key::UpArrow,
    ],
];

#[derive(Debug, Copy, Clone, PartialEq)]
enum KeyAction {
    Beep,
    Rolling,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum ButtonEvent {
    Nothing,
    Single,
    Double,
    LongPress,
}

struct KeyEvent {
    last_key_event_time: time::Instant,
    last_double_tap_time: time::Instant,
    double_click_detection_time: time::Duration,
    long_press_detection_time: time::Duration,
    next_ignore_button_event: ButtonStatus,
}

impl KeyEvent {
    fn detect_click(&mut self, mut button_info_list: Vec<ButtonInfo>) -> ButtonEvent {
        button_info_list.retain(|e| e.time >= self.last_key_event_time);

        let now = time::Instant::now();
        if button_info_list.is_empty() {
            self.last_key_event_time = now;
            return ButtonEvent::Nothing;
        }

        // Ensure that the first event of button_info_list is 'Press'
        let first_event = button_info_list[0];
        if first_event.button == ButtonStatus::Release {
            self.last_key_event_time = first_event.time + time::Duration::from_millis(1);
            button_info_list.remove(0);
            if button_info_list.is_empty() {
                return ButtonEvent::Nothing;
            }
        }

        match button_info_list.len() {
            1 => {
                if now.duration_since(self.last_key_event_time) > self.long_press_detection_time {
                    let last_event = button_info_list.last().unwrap();
                    self.last_key_event_time = last_event.time + time::Duration::from_millis(1);
                    info!(
                        "[LONG-PRESS] next ignore:{:?} buflen:{} buf:{:?}",
                        self.next_ignore_button_event,
                        button_info_list.len(),
                        now.duration_since(self.last_key_event_time)
                    );
                    return ButtonEvent::LongPress;
                } else {
                    return ButtonEvent::Nothing;
                }
            }
            2 => {
                if now.duration_since(self.last_key_event_time) > self.double_click_detection_time {
                    let last_event = button_info_list.last().unwrap();
                    self.last_key_event_time = last_event.time + time::Duration::from_millis(1);
                    info!(
                        "[SINGLE] next ignore:{:?} buflen:{} buf:{:?}",
                        self.next_ignore_button_event,
                        button_info_list.len(),
                        now.duration_since(self.last_key_event_time)
                    );
                    return ButtonEvent::Single;
                } else {
                    return ButtonEvent::Nothing;
                }
            }
            _ => {
                // Multiple click is detected
                let last_event = button_info_list.last().unwrap();
                self.last_key_event_time = last_event.time + time::Duration::from_millis(1);
                info!(
                    "[DOUBLE] next ignore:{:?} buflen:{} buf:{:?}",
                    self.next_ignore_button_event,
                    button_info_list.len(),
                    button_info_list,
                );
                return ButtonEvent::Double;
            }
        }
    }

    fn get_key_code(
        &mut self,
        key_table: KeyTableName,
        sensor_info: SensorInfo,
        click_type: ButtonEvent,
    ) -> (Option<Key>, Option<KeyAction>) {
        match click_type {
            ButtonEvent::Nothing => {
                if (self.last_double_tap_time != sensor_info.time)
                    && (sensor_info.double_tap == DoubleTapStatus::Detect)
                {
                    self.last_double_tap_time = sensor_info.time;
                    return (Some(Key::F5), Some(KeyAction::Rolling));
                } else {
                    return (None, None);
                }
            }
            ButtonEvent::Single => {
                let key = KEY_TABLE[key_table as usize][sensor_info.posture as usize];
                match key {
                    Key::Escape => (None, None),
                    x => (Some(x), None),
                }
            }
            ButtonEvent::Double => return (Some(Key::F5), None),
            ButtonEvent::LongPress => return (Some(Key::Home), Some(KeyAction::Beep)),
        }
    }
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

    let result = cube.register_norify(CoreCubeUuidName::SensorInfo, sensor_information_notify);
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
    let result = cube2.register_norify(CoreCubeUuidName::SensorInfo, sensor_information_notify);
    let sensor_handler2 = result.unwrap();

    // Register Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // MAIN LOOP
    let mut key = KeyEvent {
        last_key_event_time: time::Instant::now(),
        last_double_tap_time: time::Instant::now(),
        double_click_detection_time: time::Duration::from_millis(500),
        long_press_detection_time: time::Duration::from_millis(1500),
        next_ignore_button_event: ButtonStatus::Press,
    };

    let tick = time::Duration::from_millis(100);
    let mut last_sensor_info: SensorInfo = Default::default();
    while running.load(Ordering::SeqCst) {
        match get_sensor_info_list().pop() {
            Some(last) => last_sensor_info = last,
            None => (),
        };
        let duration = std::cmp::max(
            key.last_key_event_time.elapsed(),
            key.double_click_detection_time,
        );
        let button_info_list = get_button_info_list(duration);
        let double_click_num = key.detect_click(button_info_list);
        let (key_code, key_action) =
            key.get_key_code(key_table, last_sensor_info, double_click_num);
        match key_code {
            Some(key) => {
                info!("[KEYCODE] {:?}", key);
                let mut engio = Enigo::new();
                engio.key_down(key);
            }
            None => (),
        };
        match key_action {
            Some(action) => match action {
                KeyAction::Beep => {
                    debug!("beep");
                    let result = cube.write(
                        CoreCubeUuidName::SoundCtrl,
                        &vec![0x03, 0x01, 0x01, 0x05, 87, 0xff],
                    );
                    assert_eq!(result.unwrap(), true);
                }
                KeyAction::Rolling => {
                    debug!("rolling");
                    let result = cube.write(
                        CoreCubeUuidName::SoundCtrl,
                        &vec![
                            0x03, 0x01, 13, 15, 69, 0xff, 1, 128, 0xff, 15, 69, 0xff, 1, 128, 0xff,
                            15, 69, 0xff, 1, 128, 0xff, 50, 69, 0xff, 50, 65, 0xff, 50, 67, 0xff,
                            15, 69, 0xff, 18, 128, 0xff, 15, 67, 0xff, 70, 69, 0xff,
                        ],
                    );
                    assert_eq!(result.unwrap(), true);
                    thread::sleep(time::Duration::from_millis(3200));
                    let result = cube.write(
                        CoreCubeUuidName::MotorCtrl,
                        &vec![0x02, 0x01, 0x01, 115, 0x02, 0x02, 115, 120],
                    );
                    assert_eq!(result.unwrap(), true);
                }
            },
            None => (),
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
