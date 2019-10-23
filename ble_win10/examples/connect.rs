extern crate winrt;

use std::thread;
use std::time;

use winrt::windows::devices::bluetooth::genericattributeprofile::*;
use winrt::windows::devices::bluetooth::*;
use winrt::windows::devices::enumeration::*;
use winrt::windows::foundation::*;
use winrt::windows::storage::streams::*;
use winrt::*;

fn button_handler(
    sender: *mut GattCharacteristic,
    arg: *mut GattValueChangedEventArgs,
) -> Result<()> {
    println!("button status changed {:?} {:?}", sender, arg);

    Ok(())
}

fn main() {
    // service UUID
    let toio_uuid = Guid {
        // 10b20100-5b3b-4571-9508-cf 3e fc d7 bb ae
        Data1: 0x10b20100,
        Data2: 0x5b3b,
        Data3: 0x4571,
        Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
    };

    let toio_sound_uuid = Guid {
        // 10b20104-5b3b-4571-95 08-cf 3e fc d7 bb ae
        Data1: 0x10b20104,
        Data2: 0x5b3b,
        Data3: 0x4571,
        Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
    };

    let toio_motor_uuid = Guid {
        // 10b20102-5b3b-4571-95 08-cf 3e fc d7 bb ae
        Data1: 0x10b20102,
        Data2: 0x5b3b,
        Data3: 0x4571,
        Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
    };

    let toio_button_uuid = Guid {
        // 10b20107-5b3b-4571-95 08-cf 3e fc d7 bb ae
        Data1: 0x10b20107,
        Data2: 0x5b3b,
        Data3: 0x4571,
        Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
    };

    let toio_configuration_uuid = Guid {
        // 10b201ff-5b3b-4571-95 08-cf 3e fc d7 bb ae
        Data1: 0x10b201ff,
        Data2: 0x5b3b,
        Data3: 0x4571,
        Data4: [0x95, 0x08, 0xcf, 0x3e, 0xfc, 0xd7, 0xbb, 0xae],
    };

    // get device info from registry
    let selector = GattDeviceService::get_device_selector_from_uuid(toio_uuid).unwrap();
    let ref_selector = selector.make_reference();
    println!("ref_selector: {}", ref_selector);

    let collection = DeviceInformation::find_all_async_aqs_filter(&ref_selector)
        .unwrap()
        .blocking_get()
        .expect("find_all_async failed")
        .unwrap();
    let mut count = 1;
    println!("found: {}", collection.get_size().unwrap());

    for device_info in collection.into_iter() {
        let device_info = device_info.expect("null device_info");
        println!("-- cube {}", count);
        let id = device_info.get_id().unwrap();
        println!("id: {}", id);
        let ref_id = id.make_reference();
        count += 1;

        // connect to device
        let ble_device = BluetoothLEDevice::from_id_async(&ref_id)
            .unwrap()
            .blocking_get()
            .expect("failed to get device")
            .unwrap();

        let service = ble_device
            .get_gatt_service(toio_uuid)
            .expect("failed to get gatt service")
            .unwrap();

        // configuration control
        let configuration = service
            .get_characteristics(toio_configuration_uuid)
            .unwrap()
            .unwrap();

        // get cube version
        let conf = configuration.get_at(0).expect("error").unwrap();
        println!("description: {}", conf.get_user_description().unwrap());
        let writer = DataWriter::new();
        writer.write_bytes(&[0x01, 0x00]).expect("error");
        let buffer = writer.detach_buffer().expect("error").unwrap();

        println!("start to write_value_async()");
        let write_result = conf
            .write_value_async(&buffer)
            .unwrap()
            .blocking_get()
            .expect("failed");
        if write_result != GattCommunicationStatus::Success {
            println!("failed: write_value_async()");
            continue;
        }

        println!("start to read_value_with_cache_mode_async()");
        let read_result = conf
            .read_value_with_cache_mode_async(BluetoothCacheMode::Uncached)
            .unwrap()
            .blocking_get()
            .expect("failed: read_value_with_cache_mode_async()")
            .unwrap();

        println!("start to get_status()");

        if read_result.get_status().unwrap() == GattCommunicationStatus::Success {
            println!("read success");
            let reader = DataReader::from_buffer(&read_result.get_value().unwrap().unwrap())
                .expect("error")
                .unwrap();
            let read_length = reader.get_unconsumed_buffer_length().expect("error") as usize;
            let mut version = Vec::<u8>::with_capacity(read_length);
            for _i in 0..read_length {
                version.push(0);
            }
            reader.read_bytes(version.as_mut()).expect("error");
            println!("version: {:?} length:{}", version, read_length);
        } else {
            println!("failed: get_status()");
            continue;
        }

        // check connection
        let connected = ble_device.get_connection_status().unwrap();

        if connected == BluetoothConnectionStatus::Connected {
            println!("connected");
        } else {
            println!("not conencted");
            continue;
        }

        // get notify
        let button = service
            .get_characteristics(toio_button_uuid)
            .unwrap()
            .unwrap();

        let bt = button.get_at(0).expect("error").unwrap();

        let handler =
            <TypedEventHandler<GattCharacteristic, GattValueChangedEventArgs>>::new(button_handler);

        let token_button = bt.add_value_changed(&handler).expect("error");
        println!("token {:?}", token_button);

        bt.write_client_characteristic_configuration_descriptor_async(
            GattClientCharacteristicConfigurationDescriptorValue::Notify,
        )
        .unwrap()
        .blocking_get()
        .expect("failed");

        thread::sleep(time::Duration::from_secs(6));

        bt.write_client_characteristic_configuration_descriptor_async(
            GattClientCharacteristicConfigurationDescriptorValue::None,
        )
        .unwrap()
        .blocking_get()
        .expect("failed");

        bt.remove_value_changed(token_button).expect("error");

        // sound control
        let sound = service
            .get_characteristics(toio_sound_uuid)
            .unwrap()
            .unwrap();

        let sd = sound.get_at(0).expect("error").unwrap();
        println!("description: {}", sd.get_user_description().unwrap());

        let writer = DataWriter::new();
        writer.write_bytes(&[0x02, 9, 0xff]).expect("error");
        let buffer = writer.detach_buffer().expect("error").unwrap();
        sd.write_value_async(&buffer)
            .unwrap()
            .blocking_get()
            .expect("failed");

        // motor control
        let motor = service
            .get_characteristics(toio_motor_uuid)
            .unwrap()
            .unwrap();

        let mt = motor.get_at(0).expect("error").unwrap();
        println!("description: {}", mt.get_user_description().unwrap());
        let writer = DataWriter::new();
        writer
            .write_bytes(&[0x02, 0x01, 0x01, 0x64, 0x02, 0x02, 0x64, 0xff])
            .expect("error");
        let buffer = writer.detach_buffer().expect("error").unwrap();
        mt.write_value_async(&buffer)
            .unwrap()
            .blocking_get()
            .expect("failed");
        thread::sleep(time::Duration::from_secs(3));
    }

    println!("Hello, world!");
}
