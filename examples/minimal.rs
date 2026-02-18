use blueberry_serde::{
    deserialize_message, deserialize_packet, serialize_message, serialize_packet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SensorReading {
    sensor_id: u32,
    temperature: f32,
}

fn main() {
    let reading = SensorReading {
        sensor_id: 42,
        temperature: 23.5,
    };
    let module_key = 0x01;
    let message_key = 0x42;
    let message_bytes = serialize_message(&reading, module_key, message_key).unwrap();
    let packet_bytes = serialize_packet(&[&message_bytes]).unwrap();

    hex_dump(&packet_bytes);

    let (package_header, msgs) = deserialize_packet(&packet_bytes).unwrap();
    println!("Package header: {package_header:#?}");
    println!("Number of messages: {}", msgs.len());

    let (message_header, decoded_reading): (_, SensorReading) =
        deserialize_message(msgs[0]).unwrap();
    println!("Decoded: {decoded_reading:#?}");
    assert_eq!(reading, decoded_reading);
    assert_eq!(message_header.module_key, module_key);
    assert_eq!(message_header.message_key, message_key);
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
