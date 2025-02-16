use std::sync::{Arc, mpsc::Sender};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

use crate::traffic_light::{TrafficLightMap, can_proceed_lane};
use crate::system_monitoring::LogEvent;
use crate::lanes::{load_lanes, Lane, LaneCategory};

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

/// Internal helper for Dijkstra’s algorithm over intersections.
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

/// New function: find_lane_path computes a route based on internal lanes.
fn find_lane_path(start: u32, end: u32, lanes: &Vec<Lane>) -> Option<Vec<Lane>> {
    #[derive(Debug)]
    struct LaneState {
        cost: f64,
        position: u32,
    }
    impl Eq for LaneState {}
    impl PartialEq for LaneState {
        fn eq(&self, other: &Self) -> bool {
            self.cost == other.cost
        }
    }
    impl Ord for LaneState {
        fn cmp(&self, other: &Self) -> Ordering {
            other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
        }
    }
    impl PartialOrd for LaneState {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut dist: HashMap<u32, f64> = HashMap::new();
    let mut prev: HashMap<u32, (u32, Lane)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    
    for inter in 1..=16 {
        dist.insert(inter, std::f64::INFINITY);
    }
    dist.insert(start, 0.0);
    heap.push(LaneState { cost: 0.0, position: start });
    
    let mut lane_map: HashMap<u32, Vec<&Lane>> = HashMap::new();
    for lane in lanes {
        lane_map.entry(lane.start_intersection).or_default().push(lane);
    }
    
    while let Some(LaneState { cost, position }) = heap.pop() {
        if position == end {
            break;
        }
        if cost > dist[&position] {
            continue;
        }
        if let Some(neighbor_lanes) = lane_map.get(&position) {
            for &lane in neighbor_lanes {
                let next = lane.end_intersection;
                let next_cost = cost + lane.length;
                if next_cost < *dist.get(&next).unwrap_or(&std::f64::INFINITY) {
                    dist.insert(next, next_cost);
                    prev.insert(next, (position, lane.clone()));
                    heap.push(LaneState { cost: next_cost, position: next });
                }
            }
        }
    }
    
    if !dist.contains_key(&end) || dist[&end] == std::f64::INFINITY {
        return None;
    }
    
    let mut path: Vec<Lane> = Vec::new();
    let mut current = end;
    while current != start {
        if let Some(&(prev_inter, ref lane)) = prev.get(&current) {
            path.push(lane.clone());
            current = prev_inter;
        } else {
            break;
        }
    }
    path.reverse();
    Some(path)
}

/// Helper: returns the current system time in seconds (Unix epoch).
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Simulate a single car traveling from an input boundary lane to an output boundary lane.
pub fn simulate_car(
    car_id: u32,
    traffic_lights: TrafficLightMap,
    log_tx: Sender<LogEvent>,
    entry_lanes: &[Lane],
    exit_lanes: &[Lane],
) -> CarMetrics {
    let mut rng = rand::thread_rng();
    let speed: f64 = rng.gen_range(70.0..=90.0);
    // If your version of rand complains, you might try:
    // let speed: f64 = rng.random_range(10.0..=30.0);

    // Choose a random entry and exit lane.
    let input_lane = entry_lanes[rng.gen_range(0..entry_lanes.len())].clone();
    let mut exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    while exit_lane.id == input_lane.id {
        exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    }

    let start_intersection = input_lane.end_intersection;
    let end_intersection = exit_lane.start_intersection;

    let all_lanes = load_lanes();
    let internal_lanes: Vec<Lane> = all_lanes
        .into_iter()
        .filter(|l| l.category == LaneCategory::Internal)
        .collect();

    let lane_route = match find_lane_path(start_intersection, end_intersection, &internal_lanes) {
        Some(route) => route,
        None => Vec::new(),
    };

    let lane_ids: Vec<u32> = lane_route.iter().map(|lane| lane.id).collect();
    let gen_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Generated vehicle with speed {:.2} m/s; Entry Lane {} (Inter. {}), Exit Lane {} (Inter. {}); Lane Route: {:?}",
            speed, input_lane.id, input_lane.end_intersection, exit_lane.id, exit_lane.start_intersection, lane_ids
        ),
        timestamp: current_time_secs(),
    };
    log_tx.send(gen_log).ok();

    let start_time = Instant::now();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    // 1. Travel the entry lane.
    let travel_time = input_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(travel_time));
    total_drive_time += travel_time;

    // 2. Follow the lane route.
    for lane in lane_route {
        let wait_start = Instant::now();
        // Wait until the individual lane's light turns green.
        loop {
            let can_go = {
                let locked = traffic_lights.lock().unwrap();
                can_proceed_lane(lane.id, &*locked)
            };
            if can_go {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        total_wait_time += wait_start.elapsed().as_secs_f64();

        let seg_time = lane.length / speed;
        thread::sleep(Duration::from_secs_f64(seg_time));
        total_drive_time += seg_time;
    }

    // 3. Travel the exit lane.
    let exit_time = exit_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(exit_time));
    total_drive_time += exit_time;

    let total_time = start_time.elapsed().as_secs_f64();
    let comp_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!("Completed journey: Wait={:.2}s, Drive={:.2}s, Total={:.2}s",
                        total_wait_time, total_drive_time, total_time),
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
    traffic_lights: TrafficLightMap,
    log_tx: Sender<LogEvent>,
) {
    let (result_tx, result_rx) = std::sync::mpsc::channel();

    // 1. Load all lanes.
    let all_lanes = load_lanes();

    // 2. Filter boundary lanes.
    let entry_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::InputBoundary)
        .cloned()
        .collect();
    let exit_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::OutputBoundary)
        .cloned()
        .collect();

    // 3. Launch 30 car threads.
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

    // 4. Compute average times.
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
        message: format!("Average Times - Wait: {:.2} s, Drive: {:.2} s, Total: {:.2} s",
                         total_wait / 30.0, total_drive / 30.0, total_total / 30.0),
        timestamp: current_time_secs(),
    };
    log_tx.send(avg_log).ok();
}
