// simulation.rs

use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

use crate::traffic_light::LightState;
use crate::system_monitoring::LogEvent;
// Import our lane definitions
use crate::lanes::{load_lanes, Lane, LaneCategory};

/// Represents the four cardinal directions used by intersections and lanes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

/// Metrics recorded for each car’s trip.
pub struct CarMetrics {
    pub id: u32,
    pub wait_time: f64,
    pub drive_time: f64,
    pub total_time: f64,
}

/// A road segment (optional reference structure).
#[derive(Debug, Clone)]
pub struct RoadSegment {
    pub from: u32,
    pub to: u32,
    pub length: f64,
    pub lanes: u32,
}

/// Internal helper for Dijkstra’s algorithm.
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
    // BinaryHeap is a max-heap, so invert the comparison
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Converts intersection ID (1..16) to (row, col) in a 4×4 grid.
fn intersection_to_coords(inter: u32) -> (u32, u32) {
    let row = (inter - 1) / 4;
    let col = (inter - 1) % 4;
    (row, col)
}

/// Determines direction from intersection `from` to `to` in the grid.
fn get_travel_direction(from: u32, to: u32) -> Direction {
    let (r1, c1) = intersection_to_coords(from);
    let (r2, c2) = intersection_to_coords(to);
    if r1 == r2 {
        if c2 > c1 { Direction::East } else { Direction::West }
    } else if c1 == c2 {
        if r2 > r1 { Direction::South } else { Direction::North }
    } else {
        // Should not happen in a strictly grid-based adjacency
        Direction::North
    }
}

/// Creates a simple 4×4 grid graph:
///   - Horizontal edges = 80 m
///   - Vertical edges = 100 m
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

/// Dijkstra’s algorithm to find the shortest path of intersections from `start` to `end`.
fn find_path_dijkstra(start: u32, end: u32) -> Vec<u32> {
    let graph = create_intersection_graph();
    let mut dist: HashMap<u32, f64> = HashMap::new();
    let mut prev: HashMap<u32, u32> = HashMap::new();
    let mut heap = BinaryHeap::new();

    // Initialize
    for inter in 1..=16 {
        dist.insert(inter, std::f64::INFINITY);
    }
    dist.insert(start, 0.0);
    heap.push(State { cost: 0.0, position: start, prev: None });

    // Main loop
    while let Some(State { cost, position, .. }) = heap.pop() {
        if position == end {
            break;
        }
        if cost > dist[&position] {
            continue;
        }
        if let Some(neighbors) = graph.get(&position) {
            for &(next, length) in neighbors {
                let next_cost = cost + length;
                if next_cost < dist[&next] {
                    dist.insert(next, next_cost);
                    prev.insert(next, position);
                    heap.push(State {
                        cost: next_cost,
                        position: next,
                        prev: Some(position),
                    });
                }
            }
        }
    }

    // Reconstruct path
    let mut path = Vec::new();
    let mut current = end;
    path.push(current);
    while current != start {
        if let Some(&p) = prev.get(&current) {
            current = p;
            path.push(current);
        } else {
            // no route found
            break;
        }
    }
    path.reverse();
    path
}

/// Helper: current system time in seconds (Unix epoch).
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Simulate a single car traveling from an input boundary lane to an output boundary lane.
pub fn simulate_car(
    car_id: u32,
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: Sender<LogEvent>,
    entry_lanes: &[Lane],
    exit_lanes: &[Lane],
) -> CarMetrics {
    let mut rng = rand::thread_rng();

    // Random speed [10..30] m/s
    let speed: f64 = rng.gen_range(10.0..=30.0);

    // Randomly choose an InputBoundary lane for entry, and OutputBoundary lane for exit.
    let input_lane = entry_lanes[rng.gen_range(0..entry_lanes.len())].clone();
    let mut exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    while exit_lane.id == input_lane.id {
        // Just ensure we don't pick the same lane if it was somehow repeated
        exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    }

    let start_intersection = input_lane.intersection;
    let end_intersection = exit_lane.intersection;
    let route = find_path_dijkstra(start_intersection, end_intersection);

    // Log creation
    let gen_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Generated vehicle with speed {:.2} m/s; Entry Lane {} (Inter. {}), Exit Lane {} (Inter. {}); Route: {:?}",
            speed, input_lane.id, start_intersection, exit_lane.id, end_intersection, route
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(gen_log).ok();

    let start_time = Instant::now();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    // 1. Travel the entry lane
    let travel_time = input_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(travel_time));
    total_drive_time += travel_time;

    // 2. Follow the route intersection-to-intersection
    for window in route.windows(2) {
        let current_inter = window[0];
        let next_inter = window[1];

        // Approx distance (80m if east/west, 100m if north/south)
        let segment_dist = match get_travel_direction(current_inter, next_inter) {
            Direction::East | Direction::West => 80.0,
            Direction::North | Direction::South => 100.0,
        };

        // Wait for green light
        let wait_start = Instant::now();
        loop {
            let light_state = {
                let locked = traffic_lights.lock().unwrap();
                *locked.get(&current_inter).unwrap_or(&LightState::NSGreen)
            };
            let travel_dir = get_travel_direction(current_inter, next_inter);
            if crate::traffic_light::can_proceed(light_state, travel_dir) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        total_wait_time += wait_start.elapsed().as_secs_f64();

        // Drive the segment
        let seg_time = segment_dist / speed;
        thread::sleep(Duration::from_secs_f64(seg_time));
        total_drive_time += seg_time;
    }

    // 3. Travel the exit lane
    let exit_time = exit_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(exit_time));
    total_drive_time += exit_time;

    // Summaries
    let total_time = start_time.elapsed().as_secs_f64();
    let comp_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Completed journey: Wait={:.2}s, Drive={:.2}s, Total={:.2}s",
            total_wait_time, total_drive_time, total_time
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(comp_log).ok();

    CarMetrics {
        id: car_id,
        wait_time: total_wait_time,
        drive_time: total_drive_time,
        total_time,
    }
}

/// Spawns multiple cars, each from an InputBoundary lane to an OutputBoundary lane.
pub fn run_simulation(
    traffic_lights: Arc<Mutex<HashMap<u32, LightState>>>,
    log_tx: Sender<LogEvent>,
) {
    let (result_tx, result_rx) = std::sync::mpsc::channel();

    // 1. Load all lanes (52 total), each tagged as input, output, or internal
    let all_lanes = load_lanes();

    // 2. Filter them: only use InputBoundary as "entry", OutputBoundary as "exit"
    let entry_lanes: Vec<Lane> = all_lanes
        .iter()
        .filter(|l| l.category == LaneCategory::InputBoundary)
        .cloned()
        .collect();

    let exit_lanes: Vec<Lane> = all_lanes
        .iter()
        .filter(|l| l.category == LaneCategory::OutputBoundary)
        .cloned()
        .collect();

    // 3. Launch 30 cars
    let mut handles = vec![];
    for car_id in 1..=30 {
        let tl_clone = Arc::clone(&traffic_lights);
        let log_tx_clone = log_tx.clone();
        let result_tx_clone = result_tx.clone();
        let entry_clone = entry_lanes.clone();
        let exit_clone = exit_lanes.clone();

        let handle = thread::spawn(move || {
            let metrics = simulate_car(
                car_id,
                tl_clone,
                log_tx_clone,
                &entry_clone,
                &exit_clone,
            );
            result_tx_clone.send(metrics).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all cars to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // 4. Compute average times
    let mut total_wait = 0.0;
    let mut total_drive = 0.0;
    let mut total_total = 0.0;
    for _ in 1..=30 {
        let m = result_rx.recv().unwrap();
        total_wait += m.wait_time;
        total_drive += m.drive_time;
        total_total += m.total_time;
    }

    // 5. Log the final average
    let avg_log = LogEvent {
        source: "Simulation".to_string(),
        message: format!(
            "Average Times - Wait: {:.2} s, Drive: {:.2} s, Total: {:.2} s",
            total_wait / 30.0,
            total_drive / 30.0,
            total_total / 30.0
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(avg_log).ok();
}
