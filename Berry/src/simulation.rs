// simulation.rs
use tokio;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use serde::{Serialize, Deserialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

mod mq;
use mq::{create_channel, publish_message, declare_exchange};
use lanes::{load_lanes, Lane, LaneCategory};

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

/// Simulate a single car journey.
/// For brevity, we simulate a car by updating one lane’s count.
async fn simulate_car(
    car_id: u32,
    entry_lanes: Vec<Lane>,
    exit_lanes: Vec<Lane>,
    sim_event: Arc<Mutex<HashMap<u32, u32>>>,
    channel: &lapin::Channel,
) {
    // Use a seeded RNG (ChaCha8Rng is Send)
    let mut rng = ChaCha8Rng::from_entropy();
    let speed: f64 = rng.gen_range(70.0..=90.0);
    let input_lane = entry_lanes[rng.gen_range(0..entry_lanes.len())].clone();
    let mut exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    while exit_lane.id == input_lane.id {
        exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    }

    // Log car generation.
    let log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!("Car generated: input lane {} exit lane {}", input_lane.id, exit_lane.id),
        timestamp: current_time_secs(),
    };
    publish_message(channel, "logs", "", &log).await;

    // Simulate travel time.
    sleep(Duration::from_secs_f64(input_lane.length / speed)).await;

    // Update simulation state: increase vehicle count.
    {
        let mut stats = sim_event.lock().await;
        *stats.entry(input_lane.id).or_insert(0) += 1;
        let count = *stats.get(&input_lane.id).unwrap();
        // Drop the lock before awaiting.
        drop(stats);
        let update = TrafficUpdate {
            lane_id: input_lane.id,
            vehicle_count: count,
            timestamp: current_time_secs(),
        };
        publish_message(channel, "simulation.updates", "", &update).await;
    }

    sleep(Duration::from_secs(1)).await; // Simulate time in transit

    // Update simulation state: decrease vehicle count.
    {
        let mut stats = sim_event.lock().await;
        *stats.entry(input_lane.id).or_insert(0) -= 1;
        let count = *stats.get(&input_lane.id).unwrap();
        drop(stats);
        let update = TrafficUpdate {
            lane_id: input_lane.id,
            vehicle_count: count,
            timestamp: current_time_secs(),
        };
        publish_message(channel, "simulation.updates", "", &update).await;
    }

    // Log completion.
    let log2 = LogEvent {
        source: format!("Car-{}", car_id),
        message: "Completed journey".into(),
        timestamp: current_time_secs(),
    };
    publish_message(channel, "logs", "", &log2).await;
}

#[tokio::main]
async fn main() {
    // Set up RabbitMQ.
    let channel = create_channel().await;
    declare_exchange(&channel, "simulation.updates", lapin::ExchangeKind::Fanout).await;
    declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;

    // Shared simulation state.
    let sim_event = Arc::new(Mutex::new(HashMap::new()));

    // Load lanes.
    let all_lanes = load_lanes();
    let entry_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::InputBoundary)
        .cloned()
        .collect();
    let exit_lanes: Vec<Lane> = all_lanes.iter()
        .filter(|l| l.category == LaneCategory::OutputBoundary)
        .cloned()
        .collect();

    // Spawn simulation tasks for 30 cars.
    let mut handles = vec![];
    for car_id in 1..=30 {
        let entry_clone = entry_lanes.clone();
        let exit_clone = exit_lanes.clone();
        let sim_event_clone = Arc::clone(&sim_event);
        let channel_clone = channel.clone();
        let handle = tokio::spawn(async move {
            simulate_car(car_id, entry_clone, exit_clone, sim_event_clone, &channel_clone).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Log simulation completion.
    let log_complete = LogEvent {
        source: "Simulation".into(),
        message: "Simulation complete".into(),
        timestamp: current_time_secs(),
    };
    publish_message(&channel, "logs", "", &log_complete).await;
}
