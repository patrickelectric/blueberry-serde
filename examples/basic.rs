//! Basic usage example for blueberry-serde.
//!
//! Demonstrates serialization/deserialization of structs with various field
//! types, message headers, sequences, strings, and boolean packing.

use blueberry_serde::{deserialize, deserialize_message, serialize, serialize_message};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SensorReading {
    sensor_id: u32,
    temperature: f32,
    humidity: u16,
    alert_high: bool,
    alert_low: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DeviceStatus {
    device_id: u32,
    name: String,
    readings: Vec<u16>,
    online: bool,
    calibrated: bool,
}

fn main() {
    // -- Raw serialization (no message header) --
    println!("=== Raw Serialization ===");
    let reading = SensorReading {
        sensor_id: 42,
        temperature: 23.5,
        humidity: 65,
        alert_high: true,
        alert_low: false,
    };

    let bytes = serialize(&reading).unwrap();
    println!("Serialized {} bytes: {:02X?}", bytes.len(), bytes);

    let decoded: SensorReading = deserialize(&bytes).unwrap();
    println!("Decoded: {:?}", decoded);
    assert_eq!(reading, decoded);

    // -- Message serialization (with header) --
    println!("\n=== Message Serialization ===");
    let status = DeviceStatus {
        device_id: 100,
        name: "sensor-alpha".to_string(),
        readings: vec![1023, 2047, 4095],
        online: true,
        calibrated: false,
    };

    let module_key = 0x01;
    let message_key = 0x42;
    let msg_bytes = serialize_message(&status, module_key, message_key).unwrap();
    println!("Message {} bytes: {:02X?}", msg_bytes.len(), msg_bytes);

    let (header, decoded_status): (_, DeviceStatus) = deserialize_message(&msg_bytes).unwrap();
    println!("Header: {:?}", header);
    println!("Decoded: {:?}", decoded_status);
    assert_eq!(status, decoded_status);
    assert_eq!(header.module_key, module_key);
    assert_eq!(header.message_key, message_key);

    // -- Forward compatibility demo --
    println!("\n=== Forward Compatibility ===");

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct StatusV2 {
        device_id: u32,
        name: String,
        readings: Vec<u16>,
        online: bool,
        calibrated: bool,
        // New in V2:
        firmware_version: u32,
    }

    let new_status = StatusV2 {
        device_id: 100,
        name: "sensor-beta".to_string(),
        readings: vec![500, 600],
        online: true,
        calibrated: true,
        firmware_version: 0x0200,
    };

    let new_bytes = serialize_message(&new_status, module_key, message_key).unwrap();

    // Old firmware reads it as the original DeviceStatus (missing firmware_version)
    let (_, old_decoded): (_, DeviceStatus) = deserialize_message(&new_bytes).unwrap();
    println!(
        "Old firmware sees: device_id={}, name={:?}, online={}",
        old_decoded.device_id, old_decoded.name, old_decoded.online
    );

    println!("\nAll examples passed!");
}
