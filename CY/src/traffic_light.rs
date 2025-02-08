use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use crate::flow_analyzer::Recommendation;
use crate::system_monitoring::LogEvent;

/// Represents a traffic light at a junction.
pub struct TrafficLight {
    pub junction: u32,
    pub green_time: u32,
    pub red_time: u32,
}

/// Runs the Traffic Light Control system.
/// It listens for recommendations from the Flow Analyzer and adjusts the traffic light timings.
pub fn run_traffic_lights(rec_rx: Receiver<Recommendation>, log_tx: Sender<LogEvent>) {
    // Initialize traffic lights for 4 junctions with default timings.
    let mut lights: HashMap<u32, TrafficLight> = (1..=4)
        .map(|j| {
            (j, TrafficLight {
                junction: j,
                green_time: 30, // default green time
                red_time: 30,   // default red time (assuming a 60-second cycle)
            })
        })
        .collect();

    // Listen for recommendations and adjust timings.
    while let Ok(rec) = rec_rx.recv() {
        match rec {
            Recommendation::AdjustGreenTime { junction, new_green_time, timestamp } => {
                if let Some(light) = lights.get_mut(&junction) {
                    light.green_time = new_green_time;
                    // Adjust red time to maintain a total cycle of 60 seconds.
                    light.red_time = 60 - new_green_time;
                    let log_event = LogEvent {
                        source: format!("TrafficLight-{}", junction),
                        message: format!("Adjusted timings: Green {}s, Red {}s", light.green_time, light.red_time),
                        timestamp,
                    };
                    log_tx.send(log_event).unwrap();
                }
            }
        }
    }
}
