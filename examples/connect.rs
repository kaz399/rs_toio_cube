extern crate winrt;

use winrt::windows::devices::bluetooth::genericattributeprofile::*;
use winrt::windows::devices::enumeration::*;
use winrt::windows::storage::streams::*;
use winrt::*;

fn main() {
    let toio_uuid = Guid {
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
    let selector = GattDeviceService::get_device_selector_from_uuid(toio_uuid).unwrap();
    let ref_selector = selector.make_reference();
    println!("{}", ref_selector);
    let collection = DeviceInformation::find_all_async_aqs_filter(&ref_selector)
        .unwrap()
        .blocking_get()
        .expect("find_all_sync failed")
        .unwrap();
    let mut count = 1;
    println!("{}", collection.get_size().unwrap());
    for device_info in collection.into_iter() {
        let device_info = device_info.expect("null device_info");
        println!("{}", count);
        let id = device_info.get_id().unwrap();
        println!("{}", id);
        let ref_id = id.make_reference();
        count += 1;

        let service = GattDeviceService::from_id_async(&ref_id)
            .unwrap()
            .blocking_get()
            .expect("from_id_async failed")
            .unwrap();
        let sound = service
            .get_characteristics(toio_sound_uuid)
            .unwrap()
            .unwrap();
        for sd in sound.into_iter() {
            let sd = sd.unwrap();
            println!("{}", sd.get_user_description().unwrap());

            let writer = DataWriter::new();
            writer.write_bytes(&[0x02, 9, 0xff]).expect("error");
            let buffer = writer.detach_buffer().expect("error").unwrap();
            sd.write_value_async(&buffer)
                .unwrap()
                .blocking_get()
                .expect("failed");
        }

        let motor = service
            .get_characteristics(toio_motor_uuid)
            .unwrap()
            .unwrap();
        for mt in motor.into_iter() {
            let mt = mt.unwrap();
            println!("{}", mt.get_user_description().unwrap());
            let writer = DataWriter::new();
            writer
                .write_bytes(&[0x02, 0x01, 0x01, 0x10, 0x02, 0x01, 0x10, 0x40])
                .expect("error");
            let buffer = writer.detach_buffer().expect("error").unwrap();
            mt.write_value_async(&buffer)
                .unwrap()
                .blocking_get()
                .expect("failed");
        }
    }
    println!("Hello, world!");
}
