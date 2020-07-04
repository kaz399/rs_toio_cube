/* This is a test code */

use log::{debug, error, info};
use std::fmt::{self, Debug};
use std::sync::mpsc;
use std::time;
use winrt::*;

// Now use the `import` macro to import the desired winmd files and types:
import!(
    dependencies
      os
    types
      windows::devices::bluetooth::*
      windows::devices::bluetooth::advertisement::*
      windows::devices::bluetooth::generic_attribute_profile::*
      windows::devices::enumeration::*
      windows::foundation::*
      windows::storage::streams::*
);

use windows::devices::bluetooth::advertisement::*;
use windows::devices::bluetooth::generic_attribute_profile::*;
use windows::devices::bluetooth::*;
use windows::devices::enumeration::*;
use windows::foundation::*;
use windows::storage::streams::*;

pub type CoreCubeNotifySender = generic_attribute_profile::GattCharacteristic;
pub type CoreCubeNotifyArgs = GattValueChangedEventArgs;
pub type CoreCubeNotifyResult = Result<()>;

#[derive(Debug, Copy, Clone)]
pub enum CoreCubeUuidName {
    Service,
    IdInfo,
    SensorInfo,
    ButtonInfo,
    BatteryInfo,
    MotorCtrl,
    LightCtrl,
    SoundCtrl,
    Configuration,
}

impl fmt::Display for CoreCubeUuidName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn get_uuid(name: CoreCubeUuidName) -> Option<Guid> {
    match name {
        CoreCubeUuidName::Service => Some(Guid::from_values(
            // 10b20100-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20100,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::IdInfo => Some(Guid::from_values(
            // 10b20101-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20101,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::SensorInfo => Some(Guid::from_values(
            // 10b20106-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20106,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::ButtonInfo => Some(Guid::from_values(
            // 10b20107-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20107,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        // Characteristic: Battery information
        CoreCubeUuidName::BatteryInfo => Some(Guid::from_values(
            // 10b20108-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20108,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::MotorCtrl => Some(Guid::from_values(
            // 10b20102-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20102,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::LightCtrl => Some(Guid::from_values(
            // 10b20103-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20103,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::SoundCtrl => Some(Guid::from_values(
            // 10b20104-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b20104,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
        CoreCubeUuidName::Configuration => Some(Guid::from_values(
            // 10b201ff-5b3b-4571-9508-cf 3e fc d7 bb ae
            0x10b201ff,
            0x5b3b,
            0x4571,
            [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        )),
    }
}

pub fn get_ble_devices() -> std::result::Result<Vec<String>, String> {
    let service_uuid = get_uuid(CoreCubeUuidName::Service).unwrap();
    let selector = GattDeviceService::get_device_selector_from_uuid(service_uuid).unwrap();
    debug!("ref_selector: {}", selector);

    let collection = DeviceInformation::find_all_async_aqs_filter(&selector)
        .unwrap()
        .get()
        .expect("find_all_async failed");

    let mut uuid_list: Vec<String> = Vec::new();
    for device_info in collection.into_iter() {
        uuid_list.push(device_info.name().unwrap().to_string());
    }

    Ok(uuid_list)
}

pub fn get_ble_device_from_address(address: u64) -> std::result::Result<Vec<u64>, String> {
    info!("search with address");
    let watcher = BluetoothLEAdvertisementWatcher::new().unwrap();
    let (tx, rx) = mpsc::channel();
    let received_handler = TypedEventHandler::new(
        move |_sender: &BluetoothLEAdvertisementWatcher,
        dev_info: &BluetoothLEAdvertisementReceivedEventArgs|
        -> Result<()> {
            let dev_adrs = dev_info.bluetooth_address().unwrap();
            info!("ble address:{:16x}", address);
            if dev_adrs == address {
                tx.send(true).unwrap();
            } else {
                tx.send(false).unwrap();
            }
            Ok(())
        },
    );

    let mut found = false;
    let mut adrs_list: Vec<u64> = Vec::new();

    info!("start watcher");
    let start_time = time::Instant::now();
    watcher.received(&received_handler).unwrap();
    //watcher.add_received(&received_handler).unwrap();
    watcher.start().unwrap();
    while found == false
        && time::Instant::now().duration_since(start_time) < time::Duration::from_secs(5)
    {
        let tf = match rx.try_recv() {
            Ok(x) => x,
            Err(_) => continue,
        };
        if tf == true {
            found = true;
            adrs_list.push(address);
        }
    }
    //thread::sleep(time::Duration::from_secs(5));
    info!("stop watcher");
    watcher.stop().unwrap();

    info!("device found {}", found);
    Ok(adrs_list)
}

pub fn get_notify_data(arg: &CoreCubeNotifyArgs) -> Vec<u8> {
    let mut notify_data = Vec::<u8>::new();
    let arg_ibuffer = arg
        .characteristic_value()
        .expect("error: get_characteristic_value");

    let reader = DataReader::from_buffer(&arg_ibuffer)
        .expect("error");

    let read_length = reader.unconsumed_buffer_length().expect("error") as usize;

    for _i in 0..read_length {
        notify_data.push(0);
    }
    reader.read_bytes(notify_data.as_mut()).expect("error");
    notify_data
}

pub struct CoreCubeBLE {
    name: String,
    ble_device: Option<BluetoothLEDevice>,
    gatt_service: Option<GattDeviceService>,
}

impl Drop for CoreCubeBLE {
    fn drop(&mut self) {
        debug!("Drop: CoreCubeBLE:{}", self.name);
    }
}

pub trait CoreCubeBLEAccess {
    fn new(name: String) -> Self;

    fn connect_ref_id(&mut self, ref_id: &String) -> std::result::Result<bool, String>;
    fn connect(&mut self, address: u64) -> std::result::Result<bool, String>;
    fn read(&self, characteristic_name: CoreCubeUuidName) -> std::result::Result<Vec<u8>, String>;

    fn write(
        &self,
        characteristic_name: CoreCubeUuidName,
        bytes: &Vec<u8>,
    ) -> std::result::Result<bool, String>;

    fn register_norify(
        &self,
        characteristic_name: CoreCubeUuidName,
        handler_func: fn(
            &CoreCubeNotifySender,
            &CoreCubeNotifyArgs,
        ) -> CoreCubeNotifyResult,
    ) -> std::result::Result<CoreCubeNotifyHandler, String>;
}

impl CoreCubeBLEAccess for CoreCubeBLE {
    fn new(name: String) -> CoreCubeBLE {
        debug!("Create CoreCubeBLE: {}", name);
        CoreCubeBLE {
            name: name,
            ble_device: None,
            gatt_service: None,
        }
    }

    fn connect_ref_id(&mut self, ref_id: &String) -> std::result::Result<bool, String> {
        // connect to device
        let ref_id = HString::from(ref_id);
        let ble_device = match BluetoothLEDevice::from_id_async(&ref_id)
            .unwrap()
            .get()
        {
            Ok(bdev) => bdev,
            Err(x) => return Err(x.message()),
        };

        for gatt_service in ble_device.gatt_services().unwrap() {
            if gatt_service.uuid().unwrap() == get_uuid(CoreCubeUuidName::Service).unwrap() {
                self.gatt_service = Some(gatt_service);
            }
        }

        if self.gatt_service == None {
            return Err("Error: get_gatt_service()".to_string());
        }
        self.ble_device = Some(ble_device);

        Ok(true)
    }

    fn connect(&mut self, address: u64) -> std::result::Result<bool, String> {
        // connect to device
        info!("search with address");
        let ble_device: BluetoothLEDevice;
        let ble_device_async = BluetoothLEDevice::from_bluetooth_address_async(address).unwrap();
        ble_device = match ble_device_async.get() {
            Ok(bdev) => bdev,
            Err(_) => return Err("Error: from_id_async()".to_string()),
        };
        debug!("using IBluetoothLEDevice3 interface");
        let gatt_services = match ble_device
            .get_gatt_services_for_uuid_async(get_uuid(CoreCubeUuidName::Service).unwrap())
            .unwrap()
            .get()
        {
            Ok(s) => s,
            Err(_) => {
                error!("error: get_gatt_services_async()");
                return Err("Error: get_gatt_service_async()".to_string());
            }
        };

        //thread::sleep(time::Duration::from_secs(3));
        self.gatt_service = match gatt_services.services().unwrap().get_at(0) {
            Ok(service) => Some(service),
            Err(_) => return Err("Error: from_id_async()".to_string()),
        };
        self.ble_device = Some(ble_device);

        //dummy access
        let _chars = self.gatt_service.as_ref()
            .unwrap()
            .get_characteristics_async()
            .unwrap()
            .get()
            .unwrap();

        debug!("complete");
        Ok(true)
    }

    fn read(&self, characteristic_name: CoreCubeUuidName) -> std::result::Result<Vec<u8>, String> {
        let chr_list = self
            .gatt_service
            .clone()
            .unwrap()
            .get_characteristics_for_uuid_async(get_uuid(characteristic_name).unwrap())
            .unwrap()
            .get()
            .unwrap();
        let chr = chr_list.characteristics().unwrap().get_at(0).expect("error: read");
        let read_result = chr
            .read_value_with_cache_mode_async(BluetoothCacheMode::Uncached)
            .unwrap()
            .get()
            .expect("failed: read_value_with_cache_mode_async()");

        if read_result.status().unwrap() == GattCommunicationStatus::Success {
            let reader = DataReader::from_buffer(&read_result.value().unwrap())
                .expect("error");

            let read_length = reader.unconsumed_buffer_length().expect("error") as usize;

            let mut read_result = Vec::<u8>::with_capacity(read_length);

            for _i in 0..read_length {
                read_result.push(0);
            }
            reader.read_bytes(read_result.as_mut()).expect("error");
            return Ok(read_result);
        } else {
            error!("Error: read failed");
        }

        Err("Error: read".to_string())
    }

    fn write(
        &self,
        characteristic_name: CoreCubeUuidName,
        bytes: &Vec<u8>,
    ) -> std::result::Result<bool, String> {
        let chr_list = self
            .gatt_service
            .clone()
            .unwrap()
            .get_characteristics(get_uuid(characteristic_name).unwrap())
            .unwrap();
        let chr = chr_list.get_at(0).expect("error: read");
        let writer = DataWriter::new().unwrap();
        writer.write_bytes(bytes).expect("error");
        let buffer = writer.detach_buffer().expect("error");
        debug!("start to write_value_async()");
        let write_result = chr
            .write_value_async(&buffer)
            .unwrap()
            .get()
            .expect("failed");

        if write_result != GattCommunicationStatus::Success {
            error!("failed: write_value_async()");
            return Err("Error: write failed".to_string());
        }

        Ok(true)
    }

    fn register_norify(
        &self,
        characteristic_name: CoreCubeUuidName,
        handler_func: fn(
            &CoreCubeNotifySender,
            &CoreCubeNotifyArgs,
        ) -> CoreCubeNotifyResult,
    ) -> std::result::Result<CoreCubeNotifyHandler, String> {
        let chr_name = characteristic_name.to_string();
        let chr_list = self
            .gatt_service
            .clone()
            .unwrap()
            .get_characteristics(get_uuid(characteristic_name).unwrap())
            .unwrap();
        let chr = chr_list.get_at(0).expect("error: read");
        let winrt_handler = TypedEventHandler::new(handler_func);
        let token = Some(
            chr.value_changed(&winrt_handler.clone())
                .expect("error"),
        );
        chr.write_client_characteristic_configuration_descriptor_async(
            GattClientCharacteristicConfigurationDescriptorValue::Notify,
        )
        .unwrap()
        .get()
        .expect("failed");

        let handler = CoreCubeNotifyHandler {
            name: self.name.clone(),
            characteristic_name: chr_name.clone(),
            characteristic: chr,
            token: token,
        };

        Ok(handler)
    }
}

pub struct CoreCubeNotifyHandler {
    name: String,
    characteristic_name: String,
    characteristic: GattCharacteristic,
    token: Option<EventRegistrationToken>,
}

pub trait CoreCubeNotifyMethod {
    fn unregister(&self) -> std::result::Result<bool, String>;
}

impl CoreCubeNotifyMethod for CoreCubeNotifyHandler {
    fn unregister(&self) -> std::result::Result<bool, String> {
        match self
            .characteristic
            .write_client_characteristic_configuration_descriptor_async(
                GattClientCharacteristicConfigurationDescriptorValue::None,
            )
            .unwrap()
            .get()
        {
            Ok(_) => (),
            _ => {
                return Err(
                    "Error: write_client_characteristic_configuration_descriptor_async".to_string(),
                )
            }
        }

        match &self.token {
            Some(x) => match self.characteristic.remove_value_changed(x) {
                Ok(_) => (),
                _ => return Err("Error: remove_value_changed".to_string()),
            },
            _ => return Err("Error: token is None".to_string()),
        };

        Ok(true)
    }
}

impl Drop for CoreCubeNotifyHandler {
    fn drop(&mut self) {
        debug!(
            "Drop: CoreCubeNotifyHandler:{}:{}",
            self.name, self.characteristic_name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time;

    fn button_handler(
        sender: *mut CoreCubeNotifySender,
        arg: *mut CoreCubeNotifyArgs,
    ) -> CoreCubeNotifyResult {
        let data = get_notify_data(arg);
        println!("button status changed {:?} {:?}", sender, data);
        Ok(())
    }

    #[test]
    fn it_works() {
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
    }
}
