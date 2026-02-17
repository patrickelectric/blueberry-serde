use blueberry_serde::{deserialize, deserialize_message, serialize, serialize_message};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SensorReading {
    sensor_id: u32,
    temperature: f32,
    pressure: f64,
    flags: u8,
    enabled: bool,
    calibrated: bool,
}

#[test]
fn complex_struct_roundtrip() {
    let val = SensorReading {
        sensor_id: 42,
        temperature: 23.5,
        pressure: 101325.0,
        flags: 0x07,
        enabled: true,
        calibrated: false,
    };
    let bytes = serialize(&val).unwrap();
    let decoded: SensorReading = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn complex_struct_message_roundtrip() {
    let val = SensorReading {
        sensor_id: 42,
        temperature: 23.5,
        pressure: 101325.0,
        flags: 0x07,
        enabled: true,
        calibrated: false,
    };
    let bytes = serialize_message(&val, 0x10, 0x20).unwrap();
    let (header, decoded): (_, SensorReading) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
    assert_eq!(header.module_key, 0x10);
    assert_eq!(header.message_key, 0x20);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DeviceConfig {
    device_id: u32,
    name: String,
    channels: Vec<u16>,
    active: bool,
    debug: bool,
}

#[test]
fn struct_with_string_and_sequence() {
    let val = DeviceConfig {
        device_id: 100,
        name: "sensor-alpha".to_string(),
        channels: vec![1, 2, 3, 4],
        active: true,
        debug: false,
    };
    let bytes = serialize(&val).unwrap();
    let decoded: DeviceConfig = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn struct_with_string_and_sequence_message() {
    let val = DeviceConfig {
        device_id: 100,
        name: "sensor-alpha".to_string(),
        channels: vec![1, 2, 3, 4],
        active: true,
        debug: false,
    };
    let bytes = serialize_message(&val, 0x05, 0x01).unwrap();
    let (header, decoded): (_, DeviceConfig) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
    assert_eq!(header.module_key, 0x05);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Tuple(u32, u16, u8);

#[test]
fn tuple_struct_roundtrip() {
    let val = Tuple(1, 2, 3);
    let bytes = serialize(&val).unwrap();
    let decoded: Tuple = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn array_roundtrip() {
    let val: [u32; 4] = [10, 20, 30, 40];
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes.len(), 16);
    let decoded: [u32; 4] = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithArray {
    tag: u8,
    data: [u16; 3],
}

#[test]
fn struct_with_array() {
    let val = WithArray {
        tag: 0x42,
        data: [100, 200, 300],
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithArray = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}
