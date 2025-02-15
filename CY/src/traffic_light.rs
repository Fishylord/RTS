use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;
use std::time::Duration;

use crate::system_monitoring::LogEvent;

/// Represents the state of a traffic light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightState {
    NSGreen,
    EWGreen,
}

/// Returns true if the current light state allows movement in the specified direction.
pub fn can_proceed(state: LightState, direction: crate::simulation::Direction) -> bool {
    match state {
        LightState::NSGreen => direction == crate::simulation::Direction::North || direction == crate::simulation::Direction::South,
        LightState::EWGreen => direction == crate::simulation::Direction::East || direction == crate::simulation::Direction::West,
    }
}

/// Runs the traffic light controller.
/// Every 5 seconds the light at each intersection toggles between NSGreen and EWGreen.
pub fn run_traffic_lights(
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: Sender<LogEvent>,
) {
    loop {
        thread::sleep(Duration::from_secs(5));
        let mut lights = traffic_lights.lock().unwrap();
        for (junction, state) in lights.iter_mut() {
            *state = match *state {
                LightState::NSGreen => LightState::EWGreen,
                LightState::EWGreen => LightState::NSGreen,
            };
            let log_event = LogEvent {
                source: format!("Intersection-{}", junction),
                message: format!("Traffic light switched to {:?}", *state),
                timestamp: current_time_secs(),
            };
            log_tx.send(log_event).unwrap();
        }
    }
}

/// Helper: returns the current system time in seconds.
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
