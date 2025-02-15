use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

use crate::traffic_light::LightState;
use crate::system_monitoring::LogEvent;

/// Represents the four cardinal directions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

/// A lane along which vehicles enter or exit the simulation.
#[derive(Debug, Clone)]
pub struct Lane {
    pub id: u32,
    pub intersection: u32, // the junction this lane connects to/from
    pub direction: Direction, // travel direction for vehicles using the lane
    pub length: f64, // in meters
}

/// Metrics recorded for each car.
pub struct CarMetrics {
    pub id: u32,
    pub wait_time: f64,   // seconds spent waiting at intersections
    pub drive_time: f64,  // seconds spent moving along road segments (including entry/exit lanes)
    pub total_time: f64,  // overall journey time
}

/// A road segment between two intersections.
#[derive(Debug, Clone)]
pub struct RoadSegment {
    pub from: u32,
    pub to: u32,
    pub length: f64, // in meters
    pub lanes: u32,  // number of lanes available (for the direction from -> to)
}

/// A helper struct for Dijkstra’s algorithm.
#[derive(Debug)]
struct State {
    cost: f64,
    position: u32,
    prev: Option<u32>,
}

impl Eq for State {}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.cost.eq(&other.cost)
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that BinaryHeap is a max-heap, so we reverse the order.
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Converts an intersection ID (1..16) into (row, col) coordinates on a 4×4 grid.
fn intersection_to_coords(inter: u32) -> (u32, u32) {
    let row = (inter - 1) / 4;
    let col = (inter - 1) % 4;
    (row, col)
}

/// Determines the travel direction from one intersection to an adjacent one.
fn get_travel_direction(from: u32, to: u32) -> Direction {
    let (r1, c1) = intersection_to_coords(from);
    let (r2, c2) = intersection_to_coords(to);
    if r1 == r2 {
        if c2 > c1 { Direction::East } else { Direction::West }
    } else if c1 == c2 {
        if r2 > r1 { Direction::South } else { Direction::North }
    } else {
        // Should not occur in a grid with only horizontal/vertical moves.
        Direction::North
    }
}

/// Creates the grid graph of intersections as a mapping:
/// intersection_id -> Vec<(neighbor_intersection, road_length)>.
/// For this example:
/// • Horizontal segments (neighbors east/west) have a length of 80 m.
/// • Vertical segments (neighbors north/south) have a length of 100 m.
fn create_intersection_graph() -> HashMap<u32, Vec<(u32, f64)>> {
    let mut graph: HashMap<u32, Vec<(u32, f64)>> = HashMap::new();

    for inter in 1..=16 {
        let (row, col) = intersection_to_coords(inter);
        let mut neighbors = Vec::new();
        // North neighbor
        if row > 0 {
            neighbors.push((inter - 4, 100.0));
        }
        // South neighbor
        if row < 3 {
            neighbors.push((inter + 4, 100.0));
        }
        // West neighbor
        if col > 0 {
            neighbors.push((inter - 1, 80.0));
        }
        // East neighbor
        if col < 3 {
            neighbors.push((inter + 1, 80.0));
        }
        graph.insert(inter, neighbors);
    }
    graph
}

/// Finds the shortest path (as a list of intersections) from start to end using Dijkstra’s algorithm.
fn find_path_dijkstra(start: u32, end: u32) -> Vec<u32> {
    let graph = create_intersection_graph();
    let mut dist: HashMap<u32, f64> = HashMap::new();
    let mut prev: HashMap<u32, u32> = HashMap::new();
    let mut heap = BinaryHeap::new();

    for inter in 1..=16 {
        dist.insert(inter, std::f64::INFINITY);
    }
    dist.insert(start, 0.0);
    heap.push(State { cost: 0.0, position: start, prev: None });

    while let Some(State { cost, position, prev: _ }) = heap.pop() {
        if position == end {
            break;
        }
        if cost > *dist.get(&position).unwrap() {
            continue;
        }
        if let Some(neighbors) = graph.get(&position) {
            for &(next, length) in neighbors {
                let next_cost = cost + length;
                if next_cost < *dist.get(&next).unwrap() {
                    dist.insert(next, next_cost);
                    prev.insert(next, position);
                    heap.push(State { cost: next_cost, position: next, prev: Some(position) });
                }
            }
        }
    }

    // Reconstruct path.
    let mut path = Vec::new();
    let mut current = end;
    path.push(current);
    while current != start {
        if let Some(&p) = prev.get(&current) {
            current = p;
            path.push(current);
        } else {
            break; // no path found (should not happen in a connected grid)
        }
    }
    path.reverse();
    path
}

/// Simulates the journey of a single car through the network.
/// The car:
/// • Travels along its entry lane (length specified in the lane),
/// • Follows a computed route across intersections (using each road segment’s length),
/// • And finally travels along its exit lane.
/// At each intersection, it checks the local traffic light state (which toggles every 5 seconds)
/// and waits if its travel direction isn’t permitted.
pub fn simulate_car(
    car_id: u32,
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: Sender<LogEvent>,
    entry_lanes: &[Lane],
    exit_lanes: &[Lane],
) -> CarMetrics {
    let mut rng = rand::thread_rng();

    // Randomly select a vehicle speed between 10 and 30 m/s.
    let speed: f64 = rng.gen_range(10.0..=30.0);

    // Randomly choose an entry lane.
    let input_lane = entry_lanes[rng.gen_range(0..entry_lanes.len())].clone();
    // Randomly choose an exit lane that is different.
    let mut exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    while exit_lane.id == input_lane.id {
        exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    }

    let start_intersection = input_lane.intersection;
    let end_intersection = exit_lane.intersection;
    let route = find_path_dijkstra(start_intersection, end_intersection);

    let gen_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Generated vehicle with speed {:.2} m/s; Entry Lane {} (Inter. {}), Exit Lane {} (Inter. {}); Route: {:?}",
            speed, input_lane.id, start_intersection, exit_lane.id, end_intersection, route
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(gen_log).unwrap();

    let start_time = Instant::now();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    // 1. Travel along the entry lane.
    let travel_time = input_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(travel_time));
    total_drive_time += travel_time;

    // 2. Travel the computed route.
    for window in route.windows(2) {
        let current_inter = window[0];
        let next_inter = window[1];
        let road_length = {
            // Determine road length based on whether the move is horizontal or vertical.
            let dir = get_travel_direction(current_inter, next_inter);
            match dir {
                Direction::East | Direction::West => 80.0,
                Direction::North | Direction::South => 100.0,
            }
        };

        // At the current intersection, wait for the appropriate traffic light.
        let travel_dir = get_travel_direction(current_inter, next_inter);
        let wait_start = Instant::now();
        loop {
            let light_state = {
                let lights = traffic_lights.lock().unwrap();
                *lights.get(&current_inter).unwrap_or(&LightState::NSGreen)
            };
            if crate::traffic_light::can_proceed(light_state, travel_dir) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let wait_dur = wait_start.elapsed().as_secs_f64();
        total_wait_time += wait_dur;

        // Travel along the road segment.
        let segment_time = road_length / speed;
        thread::sleep(Duration::from_secs_f64(segment_time));
        total_drive_time += segment_time;
    }

    // 3. Travel along the exit lane.
    let exit_travel_time = exit_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(exit_travel_time));
    total_drive_time += exit_travel_time;

    let total_time = start_time.elapsed().as_secs_f64();
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

/// Returns the current system time (UNIX epoch in seconds).
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Runs the simulation by spawning 30 car threads.
/// Each car picks a random entry and exit lane from pre-defined pools.
/// When all cars finish, the average wait, drive, and total times are logged.
pub fn run_simulation(
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: std::sync::mpsc::Sender<LogEvent>,
) {
    use std::sync::mpsc;
    let (result_tx, result_rx) = mpsc::channel();

    // Define entry lanes.
    // For demonstration we assume:
    // - North boundary: intersections 1–4, lanes travel South.
    // - West boundary: intersections 1,5,9,13, lanes travel East.
    let mut entry_lanes = Vec::new();
    let mut lane_id = 1;
    for &inter in &[1, 2, 3, 4] {
        entry_lanes.push(Lane { id: lane_id, intersection: inter, direction: Direction::South, length: 100.0 });
        lane_id += 1;
    }
    for &inter in &[1, 5, 9, 13] {
        entry_lanes.push(Lane { id: lane_id, intersection: inter, direction: Direction::East, length: 100.0 });
        lane_id += 1;
    }

    // Define exit lanes.
    // For demonstration:
    // - South boundary: intersections 13–16, lanes travel North.
    // - East boundary: intersections 4,8,12,16, lanes travel West.
    let mut exit_lanes = Vec::new();
    for &inter in &[13, 14, 15, 16] {
        exit_lanes.push(Lane { id: lane_id, intersection: inter, direction: Direction::North, length: 100.0 });
        lane_id += 1;
    }
    for &inter in &[4, 8, 12, 16] {
        exit_lanes.push(Lane { id: lane_id, intersection: inter, direction: Direction::West, length: 100.0 });
        lane_id += 1;
    }

    let mut handles = vec![];

    for car_id in 1..=30 {
        let tl_clone = Arc::clone(&traffic_lights);
        let log_tx_clone = log_tx.clone();
        let result_tx_clone = result_tx.clone();
        let entry_clone = entry_lanes.clone();
        let exit_clone = exit_lanes.clone();
        let handle = thread::spawn(move || {
            let metrics = simulate_car(car_id, tl_clone, log_tx_clone, &entry_clone, &exit_clone);
            result_tx_clone.send(metrics).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Compute and log averages.
    let mut total_wait = 0.0;
    let mut total_drive = 0.0;
    let mut total_total = 0.0;
    for _ in 1..=30 {
        let m = result_rx.recv().unwrap();
        total_wait += m.wait_time;
        total_drive += m.drive_time;
        total_total += m.total_time;
    }
    let avg_log = LogEvent {
        source: "Simulation".to_string(),
        message: format!(
            "Average Times - Wait: {:.2} s, Drive: {:.2} s, Total: {:.2} s",
            total_wait / 30.0, total_drive / 30.0, total_total / 30.0
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(avg_log).unwrap();
}
