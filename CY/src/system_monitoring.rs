use std::sync::mpsc::Receiver;

/// Log events that are produced by various components of the system.
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64, // simulated time in seconds
}

/// Runs the System Monitoring and Reporting component.
/// In this prototype the logs are printed to the console.
/// In a more complete system, these logs could be stored in files or made available via an interactive CLI.
pub fn run_monitoring(log_rx: Receiver<LogEvent>) {
    while let Ok(log_event) = log_rx.recv() {
        println!("[SimTime: {}] {}: {}", log_event.timestamp, log_event.source, log_event.message);
    }
}
