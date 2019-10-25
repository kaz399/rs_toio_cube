extern crate winrt;

use std::fmt::{self, Debug};

use winrt::windows::devices::bluetooth::genericattributeprofile::*;
use winrt::windows::devices::bluetooth::*;
use winrt::windows::devices::enumeration::*;
use winrt::windows::foundation::*;
use winrt::windows::storage::streams::*;
use winrt::*;

#[derive(Debug)]
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
        CoreCubeUuidName::Service => Some(Guid {
            // 10b20100-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20100,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::IdInfo => Some(Guid {
            // 10b20101-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20101,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::SensorInfo => Some(Guid {
            // 10b20106-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20106,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::ButtonInfo => Some(Guid {
            // 10b20107-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20107,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        // Characteristic: Battery information
        CoreCubeUuidName::BatteryInfo => Some(Guid {
            // 10b20108-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20108,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::MotorCtrl => Some(Guid {
            // 10b20102-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20102,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::LightCtrl => Some(Guid {
            // 10b20103-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20103,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::SoundCtrl => Some(Guid {
            // 10b20104-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b20104,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
        CoreCubeUuidName::Configuration => Some(Guid {
            // 10b201ff-5b3b-4571-9508-cf 3e fc d7 bb ae
            Data1: 0x10b201ff,
            Data2: 0x5b3b,
            Data3: 0x4571,
            Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
        }),
    }
}

pub fn get_ble_devices() -> std::result::Result<Vec<HString>, String> {
    let service_uuid = get_uuid(CoreCubeUuidName::Service).unwrap();

    let selector = GattDeviceService::get_device_selector_from_uuid(service_uuid).unwrap();

    let ref_selector = selector.make_reference();
    println!("ref_selector: {}", ref_selector);

    let collection = DeviceInformation::find_all_async_aqs_filter(&ref_selector)
        .unwrap()
        .blocking_get()
        .expect("find_all_async failed")
        .unwrap();

    let mut uuid_list: Vec<HString> = Vec::new();
    for device_info in collection.into_iter() {
        uuid_list.push(match device_info {
            Some(x) => x.get_id().unwrap(),
            None => return Err("Error: get_id()".to_string()),
        });
    }

    Ok(uuid_list)
}

pub struct CoreCubeBLE {
    name: String,
    ble_device: Option<winrt::ComPtr<BluetoothLEDevice>>,
    gatt_service: Option<winrt::ComPtr<GattDeviceService>>,
}

impl Drop for CoreCubeBLE {
    fn drop(&mut self) {
        println!("Drop: CoreCubeBLE:{}", self.name);
    }
}

pub trait CoreCubeBLEAccess {
    fn new(name: String) -> Self;

    fn connect(&mut self, ref_id: HStringReference) -> std::result::Result<bool, String>;

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
            sender: *mut GattCharacteristic,
            arg: *mut GattValueChangedEventArgs,
        ) -> Result<()>,
    ) -> std::result::Result<CoreCubeNotifyHandler, String>;
}

impl CoreCubeBLEAccess for CoreCubeBLE {
    fn new(name: String) -> CoreCubeBLE {
        println!("Create CoreCubeBLE: {}", name);
        CoreCubeBLE {
            name: name,
            ble_device: None,
            gatt_service: None,
        }
    }

    fn connect(&mut self, ref_id: HStringReference) -> std::result::Result<bool, String> {
        // connect to device
        let ble_device = match BluetoothLEDevice::from_id_async(&ref_id)
            .unwrap()
            .blocking_get()
        {
            Ok(bdev) => bdev.unwrap(),
            Err(_) => return Err("Error: from_id_async()".to_string()),
        };

        self.gatt_service =
            match ble_device.get_gatt_service(get_uuid(CoreCubeUuidName::Service).unwrap()) {
                Ok(service) => Some(service.unwrap()),
                Err(_) => return Err("Error: get_gatt_service()".to_string()),
            };
        self.ble_device = Some(ble_device);

        Ok(true)
    }

    fn read(&self, characteristic_name: CoreCubeUuidName) -> std::result::Result<Vec<u8>, String> {
        let chr_list = self
            .gatt_service
            .clone()
            .unwrap()
            .get_characteristics(get_uuid(characteristic_name).unwrap())
            .unwrap()
            .unwrap();

        let chr = chr_list.get_at(0).expect("error: read").unwrap();

        let read_result = chr
            .read_value_with_cache_mode_async(BluetoothCacheMode::Uncached)
            .unwrap()
            .blocking_get()
            .expect("failed: read_value_with_cache_mode_async()")
            .unwrap();

        if read_result.get_status().unwrap() == GattCommunicationStatus::Success {
            println!("read success");
            let reader = DataReader::from_buffer(&read_result.get_value().unwrap().unwrap())
                .expect("error")
                .unwrap();

            let read_length = reader.get_unconsumed_buffer_length().expect("error") as usize;

            let mut read_result = Vec::<u8>::with_capacity(read_length);

            for _i in 0..read_length {
                read_result.push(0);
            }
            reader.read_bytes(read_result.as_mut()).expect("error");
            println!("version: {:?} length:{}", read_result, read_length);
            return Ok(read_result);
        } else {
            println!("Error: read failed");
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
            .unwrap()
            .unwrap();

        let chr = chr_list.get_at(0).expect("error: read").unwrap();

        let writer = DataWriter::new();

        writer.write_bytes(bytes).expect("error");

        let buffer = writer.detach_buffer().expect("error").unwrap();

        println!("start to write_value_async()");
        let write_result = chr
            .write_value_async(&buffer)
            .unwrap()
            .blocking_get()
            .expect("failed");

        if write_result != GattCommunicationStatus::Success {
            println!("failed: write_value_async()");
            return Err("Error: write failed".to_string());
        }

        Ok(true)
    }

    fn register_norify(
        &self,
        characteristic_name: CoreCubeUuidName,
        handler_func: fn(
            sender: *mut GattCharacteristic,
            arg: *mut GattValueChangedEventArgs,
        ) -> Result<()>,
    ) -> std::result::Result<CoreCubeNotifyHandler, String> {
        let chr_name = characteristic_name.to_string();
        let chr_list = self
            .gatt_service
            .clone()
            .unwrap()
            .get_characteristics(get_uuid(characteristic_name).unwrap())
            .unwrap()
            .unwrap();

        let chr = chr_list.get_at(0).expect("error: read").unwrap();

        let handler =
            <TypedEventHandler<GattCharacteristic, GattValueChangedEventArgs>>::new(handler_func);

        let token = chr.add_value_changed(&handler).expect("error");

        chr.write_client_characteristic_configuration_descriptor_async(
            GattClientCharacteristicConfigurationDescriptorValue::Notify,
        )
        .unwrap()
        .blocking_get()
        .expect("failed");

        Ok(CoreCubeNotifyHandler {
            name: format!("{}:{}", self.name, chr_name),
            characteristic: chr,
            token: token,
        })
    }
}

pub struct CoreCubeNotifyHandler {
    name: String,
    characteristic: winrt::ComPtr<GattCharacteristic>,
    token: EventRegistrationToken,
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
            .blocking_get()
        {
            Ok(_) => (),
            _ => {
                return Err(
                    "Error: write_client_characteristic_configuration_descriptor_async".to_string(),
                )
            }
        }

        match self.characteristic.remove_value_changed(self.token) {
            Ok(_) => (),
            _ => return Err("Error: remove_value_changed".to_string()),
        }

        Ok(true)
    }
}

impl Drop for CoreCubeNotifyHandler {
    fn drop(&mut self) {
        println!("Drop: CoreCubeNotifyHandler:{}", self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time;

    fn button_handler(
        sender: *mut GattCharacteristic,
        arg: *mut GattValueChangedEventArgs,
    ) -> Result<()> {
        println!("button status changed {:?} {:?}", sender, arg);
        Ok(())
    }

    #[test]
    fn it_works() {
        let dev_list = get_ble_devices().unwrap();
        assert_ne!(dev_list.len(), 0);
        let device_info = dev_list[0].make_reference();

        let mut cube = CoreCubeBLE::new("Cube1".to_string());
        let result = cube.connect(device_info);
        assert_eq!(result.unwrap(), true);

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
