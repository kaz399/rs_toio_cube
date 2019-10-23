extern crate winrt;

use std::collections::HashMap;
use std::thread;
use std::time;

use winrt::windows::devices::bluetooth::genericattributeprofile::*;
use winrt::windows::devices::bluetooth::*;
use winrt::windows::devices::enumeration::*;
use winrt::windows::foundation::*;
use winrt::windows::storage::streams::*;
use winrt::*;

#[derive(Debug)]
enum CoreCubeUuidName {
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

fn get_uuid(name: CoreCubeUuidName) -> Option<Guid> {
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

struct CoreCubeBLE {
    name: String,
    id: String,
    ble_device: winrt::ComPtr<BluetoothLEDevice>,
    gatt_service: winrt::ComPtr<GattDeviceService>,
}

impl CoreCubeBLE {
    fn connect(&mut self, ref_id: HStringReference) -> std::result::Result<bool, String> {
        // connect to device
        self.ble_device = match BluetoothLEDevice::from_id_async(&ref_id)
            .unwrap()
            .blocking_get()
        {
            Ok(bdev) => bdev.unwrap(),
            Err(_) => return Err("Error: from_id_async()".to_string()),
        };

        self.gatt_service = match self
            .ble_device
            .get_gatt_service(get_uuid(CoreCubeUuidName::Service).unwrap())
        {
            Ok(service) => service.unwrap(),
            Err(_) => return Err("Error: get_gatt_service()".to_string()),
        };
        Ok(true)
    }

    fn read(&self, characteristic_name: CoreCubeUuidName) -> std::result::Result<Vec<u8>, String> {
        let chr_list = self
            .gatt_service
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
            return Ok(true);
        }

        Err("Error: write failed".to_string())
    }

    fn register_norify(
        &self,
        characteristic_name: CoreCubeUuidName,
        handler: fn(
            sender: *mut GattCharacteristic,
            arg: *mut GattValueChangedEventArgs,
        ) -> Result<()>,
    ) -> std::result::Result<EventRegistrationToken, String> {
        let chr_list = self
            .gatt_service
            .get_characteristics(get_uuid(characteristic_name).unwrap())
            .unwrap()
            .unwrap();

        let chr = chr_list.get_at(0).expect("error: read").unwrap();

        let handler =
            <TypedEventHandler<GattCharacteristic, GattValueChangedEventArgs>>::new(handler);

        let token = chr.add_value_changed(&handler).expect("error");

        chr.write_client_characteristic_configuration_descriptor_async(
            GattClientCharacteristicConfigurationDescriptorValue::Notify,
        )
        .unwrap()
        .blocking_get()
        .expect("failed");

        Err("Error: failed to register notify".to_string())
    }

    fn unregister_notify(
        &self,
        characteristic_name: CoreCubeUuidName,
        token: EventRegistrationToken,
    ) -> std::result::Result<bool, String> {
        let chr_list = self
            .gatt_service
            .get_characteristics(get_uuid(characteristic_name).unwrap())
            .unwrap()
            .unwrap();

        let chr = chr_list.get_at(0).expect("error: read").unwrap();

        chr.write_client_characteristic_configuration_descriptor_async(
            GattClientCharacteristicConfigurationDescriptorValue::None,
        )
        .unwrap()
        .blocking_get()
        .expect("failed");

        chr.remove_value_changed(token).expect("error");

        Ok(true)
    }

    fn disconnect(&self) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
