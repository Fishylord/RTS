// simulation.rs

use tokio;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use tokio::time::{sleep, Duration};
use serde::{Serialize, Deserialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use futures_util::stream::StreamExt;

mod mq;
mod lanes; // lanes.rs must be in the same folder
use lanes::{load_lanes, Lane, LaneCategory};

mod model;
use model::LightStatus;

#[derive(Serialize, Deserialize)]
pub struct TrafficUpdate {
    pub lane_id: u32,
    pub vehicle_count: u32,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Internal helper for Dijkstra’s algorithm over intersections.
fn find_lane_path(start: u32, end: u32, lanes: &Vec<Lane>) -> Option<Vec<Lane>> {
    #[derive(Debug)]
    struct LaneState {
        cost: f64,
        position: u32,
    }
    impl Eq for LaneState {}
    impl PartialEq for LaneState {
        fn eq(&self, other: &Self) -> bool {
            self.cost.eq(&other.cost)
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

    // Initialize distances for intersections 1..16.
    for inter in 1..=16 {
        dist.insert(inter, std::f64::INFINITY);
    }
    dist.insert(start, 0.0);
    heap.push(LaneState { cost: 0.0, position: start });

    // Build a mapping from start intersection to lanes.
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

/// Shared simulation state: number of cars per lane.
pub type SimEvent = Arc<Mutex<HashMap<u32, u32>>>;

pub fn initialize_simdata() -> SimEvent {
    let mut map = HashMap::new();
    let lanes = load_lanes();
    for lane in lanes {
        map.insert(lane.id, 0);
    }
    Arc::new(Mutex::new(map))
}

/// Shared light status state: mapping from lane id to its current light status.
pub type LightStatusMap = Arc<Mutex<HashMap<u32, String>>>;

/// Listens for light status updates from the "light_status" exchange and updates the shared state.
async fn listen_for_light_statuses(channel: &lapin::Channel, light_status_map: LightStatusMap)
    -> Result<(), Box<dyn std::error::Error>>
{
    // Declare the exchange (if not already declared) and bind a temporary queue.
    channel.exchange_declare(
        "light_status",
        lapin::ExchangeKind::Fanout,
        lapin::options::ExchangeDeclareOptions::default(),
        lapin::types::FieldTable::default()
    ).await?;
    let queue = channel.queue_declare(
        "",
        lapin::options::QueueDeclareOptions::default(),
        lapin::types::FieldTable::default()
    ).await?;
    channel.queue_bind(
        queue.name().as_str(),
        "light_status",
        "",
        lapin::options::QueueBindOptions::default(),
        lapin::types::FieldTable::default()
    ).await?;
    let mut consumer = channel.basic_consume(
        queue.name().as_str(),
        "light_status_consumer",
        lapin::options::BasicConsumeOptions::default(),
        lapin::types::FieldTable::default()
    ).await?;

    println!("Simulation listening for light status updates...");
    while let Some(delivery) = consumer.next().await {
         let delivery = delivery?;
         if let Ok(light_status) = serde_json::from_slice::<LightStatus>(&delivery.data) {
             let mut statuses = light_status_map.lock().await;
             statuses.insert(light_status.lane_id, light_status.status.clone());
             println!("Simulation updated light status: {:?}", light_status);
         }
         delivery.ack(lapin::options::BasicAckOptions::default()).await?;
    }
    Ok(())
}

/// Simulates a single car's journey.
async fn simulate_car(
    car_id: u32,
    channel: &lapin::Channel,
    sim_event: SimEvent,
    light_status_map: LightStatusMap,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(42 + car_id as u64);
    let speed: f64 = rng.gen_range(70.0..=90.0);

    let all_lanes = load_lanes();
    let entry_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::InputBoundary)
        .cloned()
        .collect();
    let exit_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::OutputBoundary)
        .cloned()
        .collect();

    let input_lane = entry_lanes[rng.gen_range(0..entry_lanes.len())].clone();
    let mut exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    while exit_lane.id == input_lane.id {
        exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    }

    // Compute route through internal lanes.
    let start_intersection = input_lane.end_intersection; // For input lanes, end_intersection is the grid entry.
    let end_intersection = exit_lane.start_intersection;   // For output lanes, start_intersection is the grid exit.
    let internal_lanes: Vec<Lane> = load_lanes()
        .into_iter()
        .filter(|l| l.category == LaneCategory::Internal)
        .collect();
    let lane_route = match find_lane_path(start_intersection, end_intersection, &internal_lanes) {
        Some(route) => route,
        None => Vec::new(),
    };

    let lane_ids: Vec<u32> = lane_route.iter().map(|lane| lane.id).collect();

    // Log the generated vehicle details.
    let log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!(
            "Generated vehicle with speed {:.2} m/s; Entry Lane {} (Inter. {}), Exit Lane {} (Inter. {}); Lane Route: {:?}",
            speed,
            input_lane.id,
            input_lane.end_intersection,
            exit_lane.id,
            exit_lane.start_intersection,
            lane_ids
        ),
        timestamp: current_time_secs(),
    };
    mq::publish_message(channel, "logs", "", &log).await;

    let start_time = tokio::time::Instant::now();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    // Travel the entry lane.
    let travel_time = input_lane.length / speed;
    sleep(Duration::from_secs_f64(travel_time)).await;
    total_drive_time += travel_time;

    // Follow the lane route.
    for lane in lane_route {
        // When entering the lane, update simulation state.
        {
            let mut stats = sim_event.lock().await;
            *stats.entry(lane.id).or_insert(0) += 1;
            println!("Car {} entered lane {}", car_id, lane.id);
        }

        // Wait until the traffic light for this lane is green.
        let wait_start = tokio::time::Instant::now();
        loop {
            let status = {
                let statuses = light_status_map.lock().await;
                statuses.get(&lane.id).cloned().unwrap_or("Red".to_string())
            };
            if status == "Green" {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
        total_wait_time += wait_start.elapsed().as_secs_f64();

        let seg_time = lane.length / speed;
        sleep(Duration::from_secs_f64(seg_time)).await;
        total_drive_time += seg_time;

        // When leaving the lane, update simulation state.
        {
            let mut stats = sim_event.lock().await;
            *stats.entry(lane.id).or_insert(0) -= 1;
            println!("Car {} left lane {}", car_id, lane.id);
        }
    }

    // Travel the exit lane.
    let exit_time = exit_lane.length / speed;
    sleep(Duration::from_secs_f64(exit_time)).await;
    total_drive_time += exit_time;

    let total_time = start_time.elapsed().as_secs_f64();
    let comp_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!("Completed journey: Wait={:.2}s, Drive={:.2}s, Total={:.2}s", total_wait_time, total_drive_time, total_time),
        timestamp: current_time_secs(),
    };
    mq::publish_message(channel, "logs", "", &comp_log).await;
}

#[tokio::main]
async fn main() {
    let channel = mq::create_channel().await;
    mq::declare_exchange(&channel, "simulation.updates", lapin::ExchangeKind::Fanout).await;
    mq::declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;
    // Also declare the light_status exchange for consistency.
    mq::declare_exchange(&channel, "light_status", lapin::ExchangeKind::Fanout).await;

    let sim_event = initialize_simdata();
    // Create a shared state for holding the latest light statuses.
    let light_status_map: LightStatusMap = Arc::new(Mutex::new(HashMap::new()));

    // Spawn a task to listen for light status updates.
    let channel_clone = channel.clone();
    let light_status_map_clone = Arc::clone(&light_status_map);
    tokio::spawn(async move {
        if let Err(e) = listen_for_light_statuses(&channel_clone, light_status_map_clone).await {
            eprintln!("Error listening for light statuses: {}", e);
        }
    });

    let mut handles = vec![];
    for car_id in 1..=30 {
        let channel_clone = channel.clone();
        let sim_event_clone = Arc::clone(&sim_event);
        let light_status_map_clone = Arc::clone(&light_status_map);
        let handle = tokio::spawn(async move {
            simulate_car(car_id, &channel_clone, sim_event_clone, light_status_map_clone).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let log_complete = LogEvent {
        source: "Simulation".into(),
        message: "Simulation complete".into(),
        timestamp: current_time_secs(),
    };
    mq::publish_message(&channel, "logs", "", &log_complete).await;
}
