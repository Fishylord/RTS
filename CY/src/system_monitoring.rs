use serde::{Serialize, Deserialize};
use zmq;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

pub fn current_time_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub fn run_monitoring() {
    let context = zmq::Context::new();
    let socket = context.socket(zmq::PULL).expect("Failed to create PULL socket");
    socket.bind("tcp://*:7000").expect("Failed to bind to tcp://*:7000 for logs");
    
    println!("System Monitoring started. Listening for log events on tcp://*:7000");

    loop {
        // recv_string returns a Result<Option<String>, _> in some versions.
        match socket.recv_string(0) {
            Ok(Ok(json_str)) => {
                if let Ok(log_event) = serde_json::from_str::<LogEvent>(&json_str) {
                    println!("[Time: {}] {}: {}", log_event.timestamp, log_event.source, log_event.message);
                } else {
                    eprintln!("Failed to deserialize log event: {}", json_str);
                }
            },
            Ok(Err(e)) => {
                // Here e is a Vec<u8>; use debug formatting.
                eprintln!("Error receiving log event: {:?}", e);
            },
            Err(e) => {
                eprintln!("Socket error: {:?}", e);
            }
        }
    }
}
