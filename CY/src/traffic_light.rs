use std::sync::{Arc, Mutex, mpsc::Sender};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::system_monitoring::LogEvent;

/// The state of a traffic light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightState {
    NSGreen,
    EWGreen,
}

/// Runs the traffic light controller. Every 5 seconds the light state for each
/// intersection is toggled (NSGreen ↔ EWGreen) and a log event is sent.
pub fn run_traffic_lights(
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: Sender<LogEvent>,
) {
    loop {
        thread::sleep(Duration::from_secs(5));
        {
            let mut lights = traffic_lights.lock().unwrap();
            for (intersection, state) in lights.iter_mut() {
                // Toggle the light state.
                *state = match *state {
                    LightState::NSGreen => LightState::EWGreen,
                    LightState::EWGreen => LightState::NSGreen,
                };
                // Log the change.
                let log_event = LogEvent {
                    source: format!("Intersection-{}", intersection),
                    message: format!("Traffic light switched to {:?}", *state),
                    timestamp: current_time_secs(),
                };
                log_tx.send(log_event).unwrap();
            }
        }
    }
}

/// Helper: returns the current system time in seconds.
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
