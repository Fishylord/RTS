use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use serde::{Serialize, Deserialize};
use zmq;

use crate::traffic_light::{TrafficLightMap, can_proceed_lane};
use crate::lanes::{load_lanes, Lane, LaneCategory};

#[derive(Serialize, Deserialize, Debug)]
pub struct CarMetrics {
    pub id: u32,
    pub wait_time: f64,
    pub drive_time: f64,
    pub total_time: f64,
}

#[derive(Debug, Clone)]
pub struct RoadSegment {
    pub from: u32,
    pub to: u32,
    pub length: f64,
    pub lanes: u32,
}

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

fn intersection_to_coords(inter: u32) -> (u32, u32) {
    let row = (inter - 1) / 4;
    let col = (inter - 1) % 4;
    (row, col)
}

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

fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub type SimEvent = Arc<Mutex<HashMap<u32, u32>>>;

pub fn initialize_simdata() -> SimEvent {
    let mut map = HashMap::new();
    let lanes = load_lanes();
    for lane in lanes {
        map.insert(lane.id, 0);
    }
    Arc::new(Mutex::new(map))
}

// Helper function: creates a new log socket from the given context.
fn create_log_socket(ctx: &zmq::Context) -> zmq::Socket {
    let sock = ctx.socket(zmq::PUSH).expect("Failed to create log PUSH socket");
    sock.connect("tcp://localhost:7000").expect("Failed to connect to tcp://localhost:7000");
    sock
}

pub fn simulate_car(
    car_id: u32,
    traffic_lights: TrafficLightMap,
    entry_lanes: &[Lane],
    exit_lanes: &[Lane],
    sim_event: Arc<Mutex<HashMap<u32, u32>>>,
    ctx: &zmq::Context,
) -> CarMetrics {
    let mut rng = rand::thread_rng();
    let speed: f64 = rng.gen_range(70.0..=90.0);
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

    let lane_route = find_lane_path(start_intersection, end_intersection, &internal_lanes).unwrap_or_default();
    let lane_ids: Vec<u32> = lane_route.iter().map(|lane| lane.id).collect();

    let log_socket = create_log_socket(ctx);
    let gen_log = serde_json::json!({
        "source": format!("Car-{}", car_id),
        "message": format!("Generated vehicle with speed {:.2} m/s; Entry Lane {} (Inter. {}), Exit Lane {} (Inter. {}); Lane Route: {:?}", 
                           speed, input_lane.id, input_lane.end_intersection, exit_lane.id, exit_lane.start_intersection, lane_ids),
        "timestamp": current_time_secs()
    });
    log_socket.send(gen_log.to_string().as_bytes(), 0).expect("Failed to send log event");

    let start_time = Instant::now();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    let travel_time = input_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(travel_time));
    total_drive_time += travel_time;

    for lane in lane_route {
        {
            let mut stats = sim_event.lock().unwrap();
            *stats.entry(lane.id).or_insert(0) += 1;
            println!("car {} entered lane {}", car_id, lane.id);
        }
        
        let wait_start = Instant::now();
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
        {
            let mut stats = sim_event.lock().unwrap();
            *stats.entry(lane.id).or_insert(0) -= 1;
            println!("car {} left lane {}", car_id, lane.id);
        }
    }

    let exit_time = exit_lane.length / speed;
    thread::sleep(Duration::from_secs_f64(exit_time));
    total_drive_time += exit_time;

    let total_time = start_time.elapsed().as_secs_f64();
    let comp_log = serde_json::json!({
        "source": format!("Car-{}", car_id),
        "message": format!("Completed journey: Wait={:.2}s, Drive={:.2}s, Total={:.2}s", total_wait_time, total_drive_time, total_time),
        "timestamp": current_time_secs()
    });
    log_socket.send(comp_log.to_string().as_bytes(), 0).expect("Failed to send log event");

    CarMetrics {
        id: car_id,
        wait_time: total_wait_time,
        drive_time: total_drive_time,
        total_time,
    }
}

pub fn run_simulation(traffic_lights: TrafficLightMap) {
    let context = zmq::Context::new();
    // Create a PUSH socket for sending simulation updates.
    let sim_socket = context.socket(zmq::PUSH).expect("Failed to create simulation PUSH socket");
    sim_socket.bind("tcp://*:7001").expect("Failed to bind tcp://*:7001");

    // For logging outside of car threads.
    let log_socket = context.socket(zmq::PUSH).expect("Failed to create log PUSH socket");
    log_socket.connect("tcp://localhost:7000").expect("Failed to connect to tcp://localhost:7000");

    let sim_event: SimEvent = initialize_simdata();
    let all_lanes = load_lanes();
    let entry_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::InputBoundary)
        .cloned()
        .collect();
    let exit_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::OutputBoundary)
        .cloned()
        .collect();

    // Share the context in an Arc so car threads can create their own log sockets.
    let ctx_arc = Arc::new(context);
    let mut handles = vec![];

    for car_id in 1..=30 {
        let tl_clone = traffic_lights.clone();
        let entry_clone = entry_lanes.clone();
        let exit_clone = exit_lanes.clone();
        let sim_event_clone = sim_event.clone();
        let ctx_clone = Arc::clone(&ctx_arc);
        let handle = thread::spawn(move || {
            let car_metrics = simulate_car(car_id, tl_clone, &entry_clone, &exit_clone, sim_event_clone, &ctx_clone);
            println!("Car {} metrics: {:?}", car_id, car_metrics);
        });
        handles.push(handle);
    }

    // Spawn a thread to periodically send simulation updates.
    {
        let sim_event_sender = sim_event.clone();
        // Instead of cloning sim_socket (which is not cloneable), create a new PUSH socket from the shared context.
        let ctx_for_sim = Arc::clone(&ctx_arc);
        thread::spawn(move || {
            let sim_sock = ctx_for_sim.socket(zmq::PUSH).expect("Failed to create simulation update socket");
            // This new socket connects to the same bound endpoint.
            sim_sock.connect("tcp://localhost:7001").expect("Failed to connect to tcp://localhost:7001");
            loop {
                thread::sleep(Duration::from_secs(5));
                if let Ok(lanes) = sim_event_sender.lock() {
                    let json_data = serde_json::to_string(&*lanes).unwrap();
                    sim_sock.send(json_data.as_bytes(), 0).expect("Failed to send simulation update");
                }
            }
        });
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let avg_log = serde_json::json!({
        "source": "Simulation",
        "message": "Simulation complete.",
        "timestamp": current_time_secs()
    });
    log_socket.send(avg_log.to_string().as_bytes(), 0).expect("Failed to send log event");
}
