use std::sync::mpsc::Receiver;

/// A log event.
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

/// Runs the system monitoring component by printing log events.
pub fn run_monitoring(log_rx: Receiver<LogEvent>) {
    while let Ok(log_event) = log_rx.recv() {
        println!("[Time: {}] {}: {}", log_event.timestamp, log_event.source, log_event.message);
    }
}
