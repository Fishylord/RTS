use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::collections::{HashMap, VecDeque};

use crate::traffic_light::LightState;
use crate::system_monitoring::LogEvent;

/// The type of vehicle.
#[derive(Debug, Clone)]
pub enum VehicleType {
    Car,
    Bus,
    Truck,
}

/// Metrics recorded for each car.
pub struct CarMetrics {
    pub id: u32,
    pub wait_time: f64,   // seconds
    pub drive_time: f64,  // seconds
    pub total_time: f64,  // seconds
}

/// The “edge” from which a lane comes.
#[derive(Debug, Clone, Copy)]
pub enum Edge {
    North,
    South,
    East,
    West,
}

/// A Lane is defined by an ID and the edge of the simulation it represents.
#[derive(Debug, Clone, Copy)]
pub struct Lane {
    pub id: u32,
    pub edge: Edge,
}

/// Returns a random lane (IDs 1–24 are distributed as follows):
/// 1–6: North, 7–12: South, 13–18: East, 19–24: West.
pub fn random_lane() -> Lane {
    let mut rng = rand::thread_rng();
    let lane_id = rng.gen_range(1..=24);
    let edge = if lane_id <= 6 {
        Edge::North
    } else if lane_id <= 12 {
        Edge::South
    } else if lane_id <= 18 {
        Edge::East
    } else {
        Edge::West
    };
    Lane { id: lane_id, edge }
}

/// Map a lane to its associated intersection (for entering or exiting):
/// • North: lanes 1–3 → intersection 1; lanes 4–6 → intersection 2
/// • South: lanes 7–9 → intersection 3; lanes 10–12 → intersection 4
/// • East: lanes 13–15 → intersection 2; lanes 16–18 → intersection 4
/// • West: lanes 19–21 → intersection 1; lanes 22–24 → intersection 3
pub fn lane_to_intersection(lane: Lane) -> u32 {
    match lane.edge {
        Edge::North => {
            if lane.id <= 3 { 1 } else { 2 }
        },
        Edge::South => {
            if lane.id <= 9 { 3 } else { 4 }
        },
        Edge::East => {
            if lane.id <= 15 { 2 } else { 4 }
        },
        Edge::West => {
            if lane.id <= 21 { 1 } else { 3 }
        },
    }
}

/// A simple graph of the 4 intersections.
/// We assume intersections 1,2,3,4 with connections:
/// 1 ↔ 2, 1 ↔ 3, 2 ↔ 4, 3 ↔ 4.
fn get_intersection_graph() -> HashMap<u32, Vec<u32>> {
    let mut graph = HashMap::new();
    graph.insert(1, vec![2, 3]);
    graph.insert(2, vec![1, 4]);
    graph.insert(3, vec![1, 4]);
    graph.insert(4, vec![2, 3]);
    graph
}

/// Finds a route (list of intersections) from start to end using BFS.
fn find_path(start: u32, end: u32) -> Vec<u32> {
    if start == end {
        return vec![start];
    }
    let graph = get_intersection_graph();
    let mut queue = VecDeque::new();
    let mut came_from = HashMap::new();
    queue.push_back(start);
    came_from.insert(start, 0); // dummy value
    while let Some(current) = queue.pop_front() {
        if current == end {
            break;
        }
        if let Some(neighbors) = graph.get(&current) {
            for &next in neighbors {
                if !came_from.contains_key(&next) {
                    queue.push_back(next);
                    came_from.insert(next, current);
                }
            }
        }
    }
    // Reconstruct the path.
    let mut path = vec![end];
    let mut current = end;
    while current != start {
        if let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        } else {
            break;
        }
    }
    path.reverse();
    path
}

/// The travel direction a car takes on a given segment.
#[derive(Debug, PartialEq)]
pub enum TravelDirection {
    North,
    South,
    East,
    West,
}

/// Given two intersections (with assumed positions):
/// 1: top‑left, 2: top‑right, 3: bottom‑left, 4: bottom‑right,
/// this function returns the travel direction for the segment.
pub fn get_direction_between(inter1: u32, inter2: u32) -> Option<TravelDirection> {
    match (inter1, inter2) {
        (1, 2) => Some(TravelDirection::East),
        (1, 3) => Some(TravelDirection::South),
        (2, 1) => Some(TravelDirection::West),
        (3, 1) => Some(TravelDirection::North),
        (2, 4) => Some(TravelDirection::South),
        (4, 2) => Some(TravelDirection::North),
        (3, 4) => Some(TravelDirection::East),
        (4, 3) => Some(TravelDirection::West),
        _ => None,
    }
}

/// For an entry lane, the travel direction is simply opposite the edge:
/// • North edge → traveling South, etc.
pub fn get_entry_direction(lane: Lane) -> TravelDirection {
    match lane.edge {
        Edge::North => TravelDirection::South,
        Edge::South => TravelDirection::North,
        Edge::East  => TravelDirection::West,
        Edge::West  => TravelDirection::East,
    }
}

/// For an exit lane, use the same logic.
pub fn get_exit_direction(lane: Lane) -> TravelDirection {
    get_entry_direction(lane)
}

/// Returns true if a given traffic light state allows travel in the specified direction.
/// When NSGreen is active, North/South movement is allowed; when EWGreen, East/West.
pub fn can_proceed(light_state: LightState, direction: &TravelDirection) -> bool {
    match light_state {
        LightState::NSGreen => *direction == TravelDirection::North || *direction == TravelDirection::South,
        LightState::EWGreen => *direction == TravelDirection::East || *direction == TravelDirection::West,
    }
}

/// Simulates the journey of a single car.
/// The car:
/// • Travels along its entry lane (100 m),
/// • Follows a path (each road segment is 100 m) between intersections,
/// • And then travels along its exit lane (100 m).
/// At each intersection it checks the current traffic light (which toggles every 5 seconds)
/// and waits if its direction is not allowed.
pub fn simulate_car(
    car_id: u32,
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: Sender<LogEvent>,
) -> CarMetrics {
    let mut rng = rand::thread_rng();

    // Randomly choose a vehicle type.
    let vehicle_type = match rng.gen_range(0..3) {
        0 => VehicleType::Car,
        1 => VehicleType::Bus,
        _ => VehicleType::Truck,
    };

    // Random speed between 10 and 30 m/s.
    let speed: f64 = rng.gen_range(10.0..=30.0);

    // Randomly choose input and exit lanes (ensuring they are different).
    let input_lane = random_lane();
    let mut exit_lane = random_lane();
    while exit_lane.id == input_lane.id {
        exit_lane = random_lane();
    }

    // Determine start and end intersections.
    let start_intersection = lane_to_intersection(input_lane);
    let end_intersection = lane_to_intersection(exit_lane);

    // Find a path (list of intersections) from start to end.
    let path = find_path(start_intersection, end_intersection);

    // Log the generated car.
    let gen_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Generated {:?} with speed {:.2} m/s, input lane {:?} (intersection {}), exit lane {:?} (intersection {}), route: {:?}",
            vehicle_type, speed, input_lane, start_intersection, exit_lane, end_intersection, path
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(gen_log).unwrap();

    let start_time = Instant::now();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    // Each lane/road segment is 100 m.
    let segment_distance = 100.0;
    let travel_time_segment = segment_distance / speed;

    // Travel along the entry lane.
    thread::sleep(Duration::from_secs_f64(travel_time_segment));
    total_drive_time += travel_time_segment;

    // For each road segment between intersections along the path.
    for window in path.windows(2) {
        let current_intersection = window[0];
        let next_intersection = window[1];
        // Determine the travel direction for this segment.
        let travel_direction = get_direction_between(current_intersection, next_intersection)
            .unwrap_or(TravelDirection::North); // default if needed

        // At the current intersection, wait until the light permits travel.
        let wait_start = Instant::now();
        loop {
            let light_state = {
                let lights = traffic_lights.lock().unwrap();
                *lights.get(&current_intersection).unwrap_or(&LightState::NSGreen)
            };
            if can_proceed(light_state, &travel_direction) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let wait_duration = wait_start.elapsed().as_secs_f64();
        total_wait_time += wait_duration;

        // Travel the road segment.
        thread::sleep(Duration::from_secs_f64(travel_time_segment));
        total_drive_time += travel_time_segment;
    }

    // Travel along the exit lane.
    thread::sleep(Duration::from_secs_f64(travel_time_segment));
    total_drive_time += travel_time_segment;

    let total_time = start_time.elapsed().as_secs_f64();

    // Log the completion of the journey.
    let comp_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Completed journey: Wait Time: {:.2} s, Drive Time: {:.2} s, Total Time: {:.2} s",
            total_wait_time, total_drive_time, total_time
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(comp_log).unwrap();

    CarMetrics {
        id: car_id,
        wait_time: total_wait_time,
        drive_time: total_drive_time,
        total_time,
    }
}

/// Helper: returns the current system time in seconds (used for logging).
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Spawns 30 car threads and, after they all finish, computes average wait, drive, and total times.
pub fn run_simulation(
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: std::sync::mpsc::Sender<LogEvent>,
) {
    use std::sync::mpsc;
    let (result_tx, result_rx) = mpsc::channel();

    let mut handles = vec![];

    // Spawn 30 car threads.
    for car_id in 1..=30 {
        let tl_clone = Arc::clone(&traffic_lights);
        let log_tx_clone = log_tx.clone();
        let result_tx_clone = result_tx.clone();
        let handle = thread::spawn(move || {
            let metrics = simulate_car(car_id, tl_clone, log_tx_clone);
            result_tx_clone.send(metrics).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all car threads to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    // Collect results and compute averages.
    let mut total_wait = 0.0;
    let mut total_drive = 0.0;
    let mut total_total = 0.0;
    for _ in 1..=30 {
        let m = result_rx.recv().unwrap();
        total_wait += m.wait_time;
        total_drive += m.drive_time;
        total_total += m.total_time;
    }
    let avg_wait = total_wait / 30.0;
    let avg_drive = total_drive / 30.0;
    let avg_total = total_total / 30.0;

    let avg_log = LogEvent {
        source: "Simulation".to_string(),
        message: format!(
            "Average Wait Time: {:.2} s, Average Drive Time: {:.2} s, Average Total Time: {:.2} s",
            avg_wait, avg_drive, avg_total
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(avg_log).unwrap();
}
