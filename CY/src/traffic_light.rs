use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use zmq;

use crate::lanes::{Lane, load_lanes};
use crate::flow_analyzer::Recommendation; // use the common definition
use crate::system_monitoring::current_time_secs;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightColor {
    Red,
    Green,
}

pub fn can_proceed_lane(lane_id: u32, lights: &HashMap<u32, LightColor>) -> bool {
    if let Some(&color) = lights.get(&lane_id) {
        color == LightColor::Green
    } else {
        false
    }
}

fn intersection_to_coords(inter: u32) -> (f64, f64) {
    let row = ((inter - 1) / 4) as f64;
    let col = ((inter - 1) % 4) as f64;
    (row, col)
}

fn compute_lane_angle(lane: &Lane) -> f64 {
    if lane.start_intersection != 0 {
        let (sx, sy) = intersection_to_coords(lane.start_intersection);
        let (ex, ey) = intersection_to_coords(lane.end_intersection);
        let dx = ex - sx;
        let dy = ey - sy;
        let angle_rad = dy.atan2(dx);
        let mut angle_deg = angle_rad.to_degrees();
        if angle_deg < 0.0 {
            angle_deg += 360.0;
        }
        angle_deg
    } else {
        let (ex, ey) = intersection_to_coords(lane.end_intersection);
        if ex == 0.0 {
            90.0  
        } else if ex == 3.0 {
            270.0 
        } else if ey == 0.0 {
            0.0   
        } else if ey == 3.0 {
            180.0 
        } else {
            90.0  
        }
    }
}

fn group_lanes_by_direction(lanes: &[Lane]) -> Vec<Vec<u32>> {
    let threshold = 20.0;
    let mut groups: Vec<(f64, Vec<u32>)> = Vec::new();
    
    for lane in lanes {
        let angle = compute_lane_angle(lane);
        let mut added = false;
        for group in groups.iter_mut() {
            if (angle - group.0).abs() <= threshold {
                group.1.push(lane.id);
                group.0 = (group.0 * (group.1.len() as f64 - 1.0) + angle) / group.1.len() as f64;
                added = true;
                break;
            }
        }
        if !added {
            groups.push((angle, vec![lane.id]));
        }
    }
    
    groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    groups.into_iter().map(|(_avg, lanes)| lanes).collect()
}

pub type TrafficLightMap = Arc<Mutex<HashMap<u32, LightColor>>>;

pub fn initialize_traffic_lights() -> TrafficLightMap {
    let mut map = HashMap::new();
    let lanes = load_lanes();
    for lane in lanes {
        if lane.end_intersection != 0 {
            map.insert(lane.id, LightColor::Red);
        }
    }
    Arc::new(Mutex::new(map))
}

/// Runs the traffic light controller.
/// It spawns one thread per junction and also starts a thread to listen for recommendations.
pub fn run_traffic_lights(traffic_lights: TrafficLightMap) {
    let lanes = load_lanes();
    let mut junction_map: HashMap<u32, Vec<Lane>> = HashMap::new();

    for lane in lanes {
        if lane.end_intersection != 0 {
            junction_map.entry(lane.end_intersection).or_default().push(lane);
        }
    }

    // Spawn a thread for receiving recommendations via ZeroMQ.
    let rec_context = zmq::Context::new();
    let rec_socket = rec_context.socket(zmq::PULL).expect("Failed to create recommendation PULL socket");
    rec_socket.connect("tcp://localhost:7002").expect("Failed to connect to tcp://localhost:7002");
    thread::spawn(move || {
        loop {
            if let Ok(msg) = rec_socket.recv_string(0) {
                if let Ok(json_str) = msg {
                    println!("✅ Received Recommendation: {}", json_str);
                    // Here you could deserialize and act on the recommendation.
                }
            }
        }
    });

    // For logging, each junction thread will create its own PUSH socket.
    for (junction, lane_list) in junction_map.into_iter() {
        let groups = group_lanes_by_direction(&lane_list);
        let tl_clone = traffic_lights.clone();
        
        thread::spawn(move || {
            // Create a new ZeroMQ context (or reuse one if desired) for this thread.
            let ctx = zmq::Context::new();
            let log_socket = ctx.socket(zmq::PUSH).expect("Failed to create log PUSH socket");
            log_socket.connect("tcp://localhost:7000").expect("Failed to connect to tcp://localhost:7000");
            let mut group_index = 0;
            loop {
                let mut green_lanes = Vec::new();
                let mut red_lanes = Vec::new();

                {
                    let mut lights = tl_clone.lock().unwrap();
                    for lane in &lane_list {
                        if groups[group_index].contains(&lane.id) {
                            lights.insert(lane.id, LightColor::Green);
                            green_lanes.push(lane.id);
                        } else {
                            lights.insert(lane.id, LightColor::Red);
                            red_lanes.push(lane.id);
                        }
                    }
                }

                let log_event = crate::system_monitoring::LogEvent {
                    source: format!("Junction-{}", junction),
                    message: format!("Phase {} active: Green lanes {:?}, Red lanes {:?}", group_index, green_lanes, red_lanes),
                    timestamp: current_time_secs(),
                };
                let log_json = serde_json::to_string(&log_event).unwrap();
                log_socket.send(log_json.as_bytes(), 0).expect("Failed to send log event");

                thread::sleep(Duration::from_secs(5));

                {
                    let mut lights = tl_clone.lock().unwrap();
                    for lane in &lane_list {
                        lights.insert(lane.id, LightColor::Red);
                    }
                }
                thread::sleep(Duration::from_secs(10));

                group_index = (group_index + 1) % groups.len();
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
