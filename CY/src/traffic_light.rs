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

/// Returns true if the current light state allows movement from the current intersection to the next intersection.
/// Horizontal movement (same row) requires EWGreen, vertical movement (same column) requires NSGreen.
pub fn can_proceed(current_inter: u32, next_inter: u32, state: LightState) -> bool {
    let (r1, c1) = intersection_to_coords(current_inter);
    let (r2, c2) = intersection_to_coords(next_inter);
    if r1 == r2 {
        state == LightState::EWGreen
    } else if c1 == c2 {
        state == LightState::NSGreen
    } else {
        // For non-aligned intersections, disallow movement.
        false
    }
}

/// Helper function: converts an intersection ID (1..16) to (row, col) coordinates in a 4×4 grid.
fn intersection_to_coords(inter: u32) -> (u32, u32) {
    let row = (inter - 1) / 4;
    let col = (inter - 1) % 4;
    (row, col)
}

/// Runs the traffic light controller.
/// Every 5 seconds, each intersection's light toggles between NSGreen and EWGreen.
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
