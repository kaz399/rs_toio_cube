fn main() {
    windows::build!(
        Windows::Devices::Bluetooth::GenericAttributeProfile::*,
        Windows::Devices::Bluetooth::Advertisement::*,
        Windows::Devices::Bluetooth::{
            BluetoothConnectionStatus,
            BluetoothLEDevice,
            BluetoothCacheMode,
        },
        Windows::Devices::Enumeration::*,
        Windows::Foundation::*,
        Windows::Storage::Streams::{
            DataReader,
            DataWriter,
            IBuffer,
        }
    );
}
