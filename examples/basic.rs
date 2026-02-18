use blueberry_serde::{
    deserialize, deserialize_message, deserialize_packet, serialize, serialize_message,
    serialize_packet,
};
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
    println!("=== Raw Serialization ===");
    let reading = SensorReading {
        sensor_id: 42,
        temperature: 23.5,
        humidity: 65,
        alert_high: true,
        alert_low: false,
    };

    let bytes = serialize(&reading).unwrap();
    println!("Serialized {} bytes:", bytes.len());
    hex_dump(&bytes);

    let decoded: SensorReading = deserialize(&bytes).unwrap();
    println!("Decoded: {:?}", decoded);
    assert_eq!(reading, decoded);

    println!("\n=== Message Serialization ===");
    let module_key = 0x01;
    let message_key = 0x42;
    let message_bytes = serialize_message(&reading, module_key, message_key).unwrap();
    let packet_bytes = serialize_packet(&[&message_bytes]).unwrap();
    println!("Packet {} bytes:", packet_bytes.len());
    hex_dump(&packet_bytes);

    let status = DeviceStatus {
        device_id: 100,
        name: "sensor-alpha".to_string(),
        readings: vec![1023, 2047, 4095],
        online: true,
        calibrated: false,
    };

    let module_key = 0x01;
    let message_key = 0x42;
    let message_bytes = serialize_message(&status, module_key, message_key).unwrap();
    let packet_bytes = serialize_packet(&[&message_bytes]).unwrap();
    println!("Packet {} bytes:", packet_bytes.len());
    hex_dump(&packet_bytes);

    let (pkt_header, msgs) = deserialize_packet(&packet_bytes).unwrap();
    println!(
        "Packet: {} words, CRC=0x{:04X}",
        pkt_header.length_words, pkt_header.crc
    );

    let (header, decoded_status): (_, DeviceStatus) = deserialize_message(msgs[0]).unwrap();
    println!("Message header: {:?}", header);
    println!("Decoded: {:?}", decoded_status);
    assert_eq!(status, decoded_status);
    assert_eq!(header.module_key, module_key);
    assert_eq!(header.message_key, message_key);

    println!("\n=== Forward Compatibility ===");

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct StatusV2 {
        device_id: u32,
        name: String,
        readings: Vec<u32>,
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

    let new_msg = serialize_message(&new_status, module_key, message_key).unwrap();
    let new_packet = serialize_packet(&[&new_msg]).unwrap();

    // Old firmware reads it as the original DeviceStatus (missing firmware_version)
    let (_, new_msgs) = deserialize_packet(&new_packet).unwrap();
    let (_, old_decoded): (_, DeviceStatus) = deserialize_message(new_msgs[0]).unwrap();
    println!(
        "Old firmware sees: device_id={}, name={:?}, online={}",
        old_decoded.device_id, old_decoded.name, old_decoded.online
    );

    // -- Packet framing (multiple messages) --
    println!("\n=== Packet Framing ===");
    let msg1 = serialize_message(&reading, 0x01, 0x01).unwrap();
    let msg2 = serialize_message(&status, 0x01, 0x42).unwrap();

    let packet = serialize_packet(&[&msg1, &msg2]).unwrap();
    println!("Packet {} bytes:", packet.len());
    hex_dump(&packet);

    let (_, messages) = deserialize_packet(&packet).unwrap();
    let (_, r): (_, SensorReading) = deserialize_message(messages[0]).unwrap();
    let (_, s): (_, DeviceStatus) = deserialize_message(messages[1]).unwrap();
    assert_eq!(r, reading);
    assert_eq!(s, status);
}

fn hex_dump(bytes: &[u8]) {
    let cols = 16;

    let headers: Vec<String> = (0..cols)
        .map(|i| format!("{:>4}", format!("+{:X}", i)))
        .collect();
    println!("        {}", headers.join(" "));

    let full_sep = vec!["----"; cols].join(" ");
    println!("        {}", full_sep);

    for (i, chunk) in bytes.chunks(cols).enumerate() {
        let offset = i * cols;
        let mut cells: Vec<String> = chunk
            .iter()
            .map(|b| format!("{:>4}", format!("{:02X}", b)))
            .collect();
        cells.resize(cols, "    ".to_string());
        println!("0x{:02X}  | {} |", offset, cells.join(" "));
    }

    let last_count = if bytes.is_empty() {
        0
    } else if bytes.len() % cols == 0 {
        cols
    } else {
        bytes.len() % cols
    };
    println!("        {}", vec!["----"; last_count].join(" "));
}
