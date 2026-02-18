use blueberry_serde::{
    crc16_ccitt, deserialize_message, deserialize_packet, empty_message, serialize_message,
    serialize_packet, Error, MessageHeader, PacketHeader, BLUEBERRY_PORT, HEADER_SIZE,
    PACKET_HEADER_SIZE, PACKET_MAGIC,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SensorData {
    sensor_id: u32,
    value: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DeviceInfo {
    device_id: u32,
    flags: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SensorReading {
    sensor_id: u32,
    temperature: f32,
    humidity: u16,
    alert_high: bool,
    alert_low: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DeviceStatusU16Readings {
    device_id: u32,
    name: String,
    readings: Vec<u16>,
    online: bool,
    calibrated: bool,
}

#[test]
fn sensor_reading_packet_matches_expected_wire_bytes() {
    let reading = SensorReading {
        sensor_id: 42,
        temperature: 23.5,
        humidity: 65,
        alert_high: true,
        alert_low: false,
    };

    let message = serialize_message(&reading, 0x01, 0x42).unwrap();
    let packet = serialize_packet(&[&message]).unwrap();

    let expected: Vec<u8> = vec![
        0x42, 0x6c, 0x75, 0x65, 0x07, 0x00, 0xff, 0x9b, 0x42, 0x00, 0x01, 0x00, 0x05, 0x00, 0x07,
        0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0xbc, 0x41, 0x41, 0x00, 0x01, 0x00,
    ];

    assert_eq!(packet, expected);
}

#[test]
fn device_status_packet_matches_expected_wire_bytes() {
    let status = DeviceStatusU16Readings {
        device_id: 100,
        name: "sensor-alpha".to_string(),
        readings: vec![1023, 2047, 4095],
        online: true,
        calibrated: false,
    };

    let message = serialize_message(&status, 0x01, 0x42).unwrap();
    let packet = serialize_packet(&[&message]).unwrap();

    let expected: Vec<u8> = vec![
        0x42, 0x6c, 0x75, 0x65, 0x0e, 0x00, 0x72, 0xf4, 0x42, 0x00, 0x01, 0x00, 0x0c, 0x00, 0x07,
        0x00, 0x64, 0x00, 0x00, 0x00, 0x14, 0x00, 0x24, 0x00, 0x02, 0x00, 0x01, 0x00, 0x0c, 0x00,
        0x00, 0x00, 0x73, 0x65, 0x6e, 0x73, 0x6f, 0x72, 0x2d, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x03,
        0x00, 0x00, 0x00, 0xff, 0x03, 0xff, 0x07, 0xff, 0x0f, 0x00, 0x00,
    ];

    assert_eq!(packet, expected);
}

#[test]
fn packet_single_message_roundtrip() {
    let data = SensorData {
        sensor_id: 42,
        value: 1023,
    };
    let msg = serialize_message(&data, 0x01, 0x02).unwrap();
    let packet = serialize_packet(&[&msg]).unwrap();

    let (pkt_hdr, messages) = deserialize_packet(&packet).unwrap();
    assert_eq!(messages.len(), 1);

    let (msg_hdr, decoded): (_, SensorData) = deserialize_message(messages[0]).unwrap();
    assert_eq!(decoded, data);
    assert_eq!(msg_hdr.module_key, 0x01);
    assert_eq!(msg_hdr.message_key, 0x02);
    assert_eq!(pkt_hdr.length_words as usize * 4, packet.len());
}

#[test]
fn packet_multiple_messages_roundtrip() {
    let sensor = SensorData {
        sensor_id: 1,
        value: 500,
    };
    let device = DeviceInfo {
        device_id: 100,
        flags: 0xFF,
    };

    let msg1 = serialize_message(&sensor, 0x01, 0x01).unwrap();
    let msg2 = serialize_message(&device, 0x02, 0x01).unwrap();
    let packet = serialize_packet(&[&msg1, &msg2]).unwrap();

    let (_, messages) = deserialize_packet(&packet).unwrap();
    assert_eq!(messages.len(), 2);

    let (hdr1, decoded_sensor): (_, SensorData) = deserialize_message(messages[0]).unwrap();
    assert_eq!(decoded_sensor, sensor);
    assert_eq!(hdr1.module_key, 0x01);

    let (hdr2, decoded_device): (_, DeviceInfo) = deserialize_message(messages[1]).unwrap();
    assert_eq!(decoded_device, device);
    assert_eq!(hdr2.module_key, 0x02);
}

#[test]
fn packet_is_multiple_of_4_bytes() {
    let data = SensorData {
        sensor_id: 1,
        value: 2,
    };
    let msg = serialize_message(&data, 0x01, 0x01).unwrap();
    let packet = serialize_packet(&[&msg]).unwrap();

    assert_eq!(packet.len() % 4, 0);
}

#[test]
fn packet_starts_with_magic() {
    let msg = empty_message(0x01, 0x01);
    let packet = serialize_packet(&[&msg]).unwrap();

    assert_eq!(&packet[0..4], &PACKET_MAGIC);
    assert_eq!(packet[0], b'B');
    assert_eq!(packet[1], b'l');
    assert_eq!(packet[2], b'u');
    assert_eq!(packet[3], b'e');
}

#[test]
fn packet_length_in_words() {
    let msg = empty_message(0x01, 0x01);
    let packet = serialize_packet(&[&msg]).unwrap();

    let pkt_hdr = PacketHeader::decode(&packet).unwrap();
    assert_eq!(pkt_hdr.length_words as usize * 4, packet.len());
}

#[test]
fn packet_crc_validates() {
    let data = SensorData {
        sensor_id: 99,
        value: 42,
    };
    let msg = serialize_message(&data, 0x01, 0x02).unwrap();
    let packet = serialize_packet(&[&msg]).unwrap();

    let message_data = &packet[PACKET_HEADER_SIZE..];
    let pkt_hdr = PacketHeader::decode(&packet).unwrap();
    assert_eq!(pkt_hdr.crc, crc16_ccitt(message_data));
}

#[test]
fn packet_crc_mismatch_detected() {
    let msg = empty_message(0x01, 0x01);
    let mut packet = serialize_packet(&[&msg]).unwrap();

    // Corrupt one byte of message data
    packet[PACKET_HEADER_SIZE] ^= 0xFF;

    let result = deserialize_packet(&packet);
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::CrcMismatch { .. } => {}
        other => panic!("expected CrcMismatch, got: {other}"),
    }
}

#[test]
fn packet_bad_magic_rejected() {
    let msg = empty_message(0x01, 0x01);
    let mut packet = serialize_packet(&[&msg]).unwrap();

    // Corrupt magic
    packet[0] = 0x00;

    let result = deserialize_packet(&packet);
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidPacketHeader => {}
        other => panic!("expected InvalidPacketHeader, got: {other}"),
    }
}

#[test]
fn empty_message_is_header_only() {
    let msg = empty_message(0x01, 0x42);
    assert_eq!(msg.len(), HEADER_SIZE);

    let hdr = MessageHeader::decode(&msg).unwrap();
    assert_eq!(hdr.module_key, 0x01);
    assert_eq!(hdr.message_key, 0x42);
    assert_eq!(hdr.length as usize * 4, HEADER_SIZE);
    assert_eq!(hdr.max_ordinal, 2); // header ordinals only (0..2)
    assert_eq!(hdr.tbd, 0);
}

#[test]
fn empty_message_in_packet() {
    let msg = empty_message(0x01, 0x42);
    let packet = serialize_packet(&[&msg]).unwrap();

    let (_, messages) = deserialize_packet(&packet).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].len(), HEADER_SIZE);

    let hdr = MessageHeader::decode(messages[0]).unwrap();
    assert_eq!(hdr.module_key, 0x01);
    assert_eq!(hdr.message_key, 0x42);
}

#[test]
fn mixed_empty_and_populated_messages() {
    let request = empty_message(0x01, 0x10);
    let data = SensorData {
        sensor_id: 5,
        value: 999,
    };
    let response = serialize_message(&data, 0x01, 0x10).unwrap();

    let packet = serialize_packet(&[&request, &response]).unwrap();
    let (_, messages) = deserialize_packet(&packet).unwrap();
    assert_eq!(messages.len(), 2);

    // First message is empty (header only)
    let hdr0 = MessageHeader::decode(messages[0]).unwrap();
    assert_eq!(hdr0.length as usize * 4, HEADER_SIZE);

    // Second message has data
    let (_, decoded): (_, SensorData) = deserialize_message(messages[1]).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn packet_with_vec_messages() {
    let msg1 = serialize_message(
        &SensorData {
            sensor_id: 1,
            value: 10,
        },
        0x01,
        0x01,
    )
    .unwrap();
    let msg2 = serialize_message(
        &SensorData {
            sensor_id: 2,
            value: 20,
        },
        0x01,
        0x02,
    )
    .unwrap();

    // serialize_packet accepts &[Vec<u8>] thanks to AsRef<[u8]>
    let messages = vec![msg1, msg2];
    let packet = serialize_packet(&messages).unwrap();

    let (_, parsed) = deserialize_packet(&packet).unwrap();
    assert_eq!(parsed.len(), 2);
}

#[test]
fn blueberry_port_constant() {
    assert_eq!(BLUEBERRY_PORT, 16962);
    assert_eq!(BLUEBERRY_PORT, 0x4242);
}

#[test]
fn packet_header_size_constant() {
    assert_eq!(PACKET_HEADER_SIZE, 8);
}

#[test]
fn three_messages_in_one_packet() {
    let m1 = serialize_message(
        &SensorData {
            sensor_id: 1,
            value: 100,
        },
        0x01,
        0x01,
    )
    .unwrap();
    let m2 = serialize_message(
        &DeviceInfo {
            device_id: 2,
            flags: 0x0F,
        },
        0x02,
        0x01,
    )
    .unwrap();
    let m3 = empty_message(0x03, 0x01);

    let packet = serialize_packet(&[m1.as_slice(), m2.as_slice(), m3.as_slice()]).unwrap();
    assert_eq!(packet.len() % 4, 0);

    let (_, messages) = deserialize_packet(&packet).unwrap();
    assert_eq!(messages.len(), 3);

    let (h1, d1): (_, SensorData) = deserialize_message(messages[0]).unwrap();
    assert_eq!(d1.sensor_id, 1);
    assert_eq!(h1.module_key, 0x01);

    let (h2, d2): (_, DeviceInfo) = deserialize_message(messages[1]).unwrap();
    assert_eq!(d2.device_id, 2);
    assert_eq!(h2.module_key, 0x02);

    let h3 = MessageHeader::decode(messages[2]).unwrap();
    assert_eq!(h3.module_key, 0x03);
    assert_eq!(h3.length as usize * 4, HEADER_SIZE);
}
