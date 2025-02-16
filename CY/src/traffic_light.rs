use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;
use std::time::Duration;

use crate::system_monitoring::LogEvent;
use crate::lanes::{Lane, load_lanes};

/// New traffic light color for individual lane control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightColor {
    Red,
    Green,
}

/// Checks whether a given lane’s light (keyed by lane id) is green.
pub fn can_proceed_lane(lane_id: u32, lights: &HashMap<u32, LightColor>) -> bool {
    if let Some(&color) = lights.get(&lane_id) {
        color == LightColor::Green
    } else {
        false
    }
}

/// Helper: Converts an intersection ID (1..16) to (row, col) coordinates in a 4×4 grid.
fn intersection_to_coords(inter: u32) -> (f64, f64) {
    let row = ((inter - 1) / 4) as f64;
    let col = ((inter - 1) % 4) as f64;
    (row, col)
}

/// Computes the approach angle (in degrees) for a lane approaching its junction.
/// For lanes with a valid start intersection (non-boundary internal lanes),
/// we compute the vector from the start to the end intersection.
/// For input lanes (with start_intersection == 0), we assign a default angle
/// based on the junction’s location.
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
        // For input lanes, assign a default based on junction location.
        let (ex, ey) = intersection_to_coords(lane.end_intersection);
        if ex == 0.0 {
            90.0  // Top row: coming from north
        } else if ex == 3.0 {
            270.0 // Bottom row: coming from south
        } else if ey == 0.0 {
            0.0   // Left column: coming from west
        } else if ey == 3.0 {
            180.0 // Right column: coming from east
        } else {
            90.0  // Default
        }
    }
}

/// Groups lanes (entering the same junction) by similar approach angles.
/// Lanes whose computed angles differ by less than a threshold (e.g. 20°)
/// are grouped together so that they can safely have green simultaneously.
fn group_lanes_by_direction(lanes: &[Lane]) -> Vec<Vec<u32>> {
    let threshold = 20.0; // degrees tolerance
    let mut groups: Vec<(f64, Vec<u32>)> = Vec::new(); // (average angle, list of lane ids)
    
    for lane in lanes {
        let angle = compute_lane_angle(lane);
        let mut added = false;
        for group in groups.iter_mut() {
            if (angle - group.0).abs() <= threshold {
                group.1.push(lane.id);
                // Update the group’s average angle (simple average).
                group.0 = (group.0 * (group.1.len() as f64 - 1.0) + angle) / group.1.len() as f64;
                added = true;
                break;
            }
        }
        if !added {
            groups.push((angle, vec![lane.id]));
        }
    }
    
    // Sort groups by average angle and return only the lane id lists.
    groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    groups.into_iter().map(|(_avg, lanes)| lanes).collect()
}

/// Shared traffic lights mapping: key is lane id, value is LightColor.
pub type TrafficLightMap = Arc<Mutex<HashMap<u32, LightColor>>>;

/// Initializes the traffic lights for all lanes that end at a junction (i.e. require control).
/// All lights are initialized to Red so that not all are green at the start.
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

/// Runs the traffic light controller. For each junction, the controller:
///   - Identifies all lanes that enter that junction.
///   - Groups lanes by their approach direction (using compute_lane_angle).
///   - Cycles through each group in a round-robin fashion, setting the group’s lanes to green
///     (and all others at that junction to red) for a fixed time slot,
///     with a brief all-red clearance interval.
/// This design avoids conflicts and (if lanes are parallel) allows nonconflicting movements simultaneously.
pub fn run_traffic_lights(traffic_lights: TrafficLightMap, log_tx: Sender<LogEvent>) {
    // Build a mapping from junction to the lanes entering that junction.
    let lanes = load_lanes();
    let mut junction_map: HashMap<u32, Vec<Lane>> = HashMap::new();
    for lane in lanes {
        if lane.end_intersection != 0 {
            junction_map.entry(lane.end_intersection).or_default().push(lane);
        }
    }
    
    // Spawn a controller thread for each junction.
    // Use into_iter() to move ownership into the loop to satisfy 'static requirements.
    for (junction, lane_list) in junction_map.into_iter() {
        let groups = group_lanes_by_direction(&lane_list);
        let traffic_lights_clone = Arc::clone(&traffic_lights);
        let log_tx_clone = log_tx.clone();
        thread::spawn(move || {
            let mut group_index = 0;
            loop {
                {
                    let mut lights = traffic_lights_clone.lock().unwrap();
                    // For this junction, set lanes in the current group to Green, others to Red.
                    for lane in lane_list.iter() {
                        if groups[group_index].contains(&lane.id) {
                            lights.insert(lane.id, LightColor::Green);
                        } else {
                            lights.insert(lane.id, LightColor::Red);
                        }
                    }
                }
                // Log the phase change.
                let log_event = LogEvent {
                    source: format!("Junction-{}", junction),
                    message: format!("Phase {} active: lanes {:?} green", group_index, groups[group_index]),
                    timestamp: current_time_secs(),
                };
                log_tx_clone.send(log_event).unwrap();
                
                // Green phase duration.
                thread::sleep(Duration::from_secs(5));
                
                // Brief all-red clearance interval.
                {
                    let mut lights = traffic_lights_clone.lock().unwrap();
                    for lane in lane_list.iter() {
                        lights.insert(lane.id, LightColor::Red);
                    }
                }
                thread::sleep(Duration::from_secs(10));
                
                // Move to the next group.
                group_index = (group_index + 1) % groups.len();
            }
        });
    }
}

/// Helper: Returns the current system time (in seconds since the Unix epoch).
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
