// traffic_light.rs

use tokio::time::{sleep, Duration};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use futures_util::stream::StreamExt;

mod mq;
use mq::{create_channel, declare_exchange, publish_message};
mod lanes;
use lanes::{load_lanes, Lane};
use tokio;
use lapin::ExchangeKind;
use rand::Rng;
use std::error::Error;
use serde_json;

mod model;
use model::LightStatus;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LightColor {
    Red,
    Green,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Recommendation {
    pub lane_id: u32,
    pub new_green_time: u32,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

// Helper: returns the current system time in seconds.
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

// Helper: converts an intersection ID (1..16) to (row, col) coordinates in a 4×4 grid.
fn intersection_to_coords(inter: u32) -> (f64, f64) {
    let row = ((inter - 1) / 4) as f64;
    let col = ((inter - 1) % 4) as f64;
    (row, col)
}

// Helper: computes the approach angle (in degrees) for a lane approaching its junction.
fn compute_lane_angle(lane: &Lane) -> f64 {
    if lane.start_intersection != 0 {
        let (sx, sy) = intersection_to_coords(lane.start_intersection);
        let (ex, ey) = intersection_to_coords(lane.end_intersection);
        let dx = ex - sx;
        let dy = ey - sy;
        let mut angle_deg = dy.atan2(dx).to_degrees();
        if angle_deg < 0.0 {
            angle_deg += 360.0;
        }
        angle_deg
    } else {
        // For input lanes, assign a default based on junction location.
        let (ex, _) = intersection_to_coords(lane.end_intersection);
        if ex == 0.0 {
            90.0  // Top row: coming from north
        } else if ex == 3.0 {
            270.0 // Bottom row: coming from south
        } else {
            90.0  // Default
        }
    }
}

// Helper: groups lanes (entering the same junction) by similar approach angles.
// Lanes whose computed angles differ by less than a threshold (20°) are grouped.
fn group_lanes_by_direction(lanes: &[Lane]) -> Vec<Vec<u32>> {
    let threshold = 20.0;
    let mut groups: Vec<(f64, Vec<u32>)> = Vec::new(); // (average angle, list of lane ids)
    
    for lane in lanes {
        let angle = compute_lane_angle(lane);
        let mut added = false;
        for group in groups.iter_mut() {
            if (angle - group.0).abs() <= threshold {
                group.1.push(lane.id);
                // Update the group's average angle.
                group.0 = (group.0 * (group.1.len() as f64 - 1.0) + angle) / (group.1.len() as f64);
                added = true;
                break;
            }
        }
        if !added {
            groups.push((angle, vec![lane.id]));
        }
    }
    
    groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    groups.into_iter().map(|(_avg, ids)| ids).collect()
}

/// Shared traffic lights mapping: key is lane id, value is LightColor.
pub type TrafficLightMap = Arc<Mutex<HashMap<u32, LightColor>>>;

/// Initializes the traffic lights for all lanes that end at a junction.
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

/// Runs the traffic light controller:
/// - For each junction, it spawns an async task that cycles through lane groups in round-robin fashion.
/// - It logs each phase, waits 5 seconds for green and 10 seconds for all-red clearance.
/// - Concurrently, it listens for recommendations via RabbitMQ.
pub async fn run_traffic_lights() -> Result<(), Box<dyn Error>> {
    let channel = create_channel().await;
    declare_exchange(&channel, "logs", ExchangeKind::Fanout).await;
    declare_exchange(&channel, "recommendations", ExchangeKind::Fanout).await;
    // Declare a new exchange for light status updates.
    declare_exchange(&channel, "light_status", ExchangeKind::Fanout).await;

    let traffic_lights = initialize_traffic_lights();

    // Build a map: junction -> list of lanes that enter that junction.
    let lanes = load_lanes();
    let mut junction_map: HashMap<u32, Vec<Lane>> = HashMap::new();
    for lane in lanes {
        if lane.end_intersection != 0 {
            junction_map.entry(lane.end_intersection).or_default().push(lane);
        }
    }
    
    // For each junction, spawn an asynchronous task for round-robin phase cycling.
    for (junction, lane_list) in junction_map.into_iter() {
        let groups = group_lanes_by_direction(&lane_list);
        let tl_clone = Arc::clone(&traffic_lights);
        let channel_clone = channel.clone();
        tokio::spawn(async move {
            let mut group_index = 0;
            loop {
                let mut green_lanes = Vec::new();
                let mut red_lanes = Vec::new();
                {
                    let mut lights = tl_clone.lock().await;
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
                // After updating, publish the light status for each lane.
                for lane in &lane_list {
                    let status = {
                        let lights = tl_clone.lock().await;
                        match lights.get(&lane.id) {
                            Some(LightColor::Green) => "Green",
                            _ => "Red",
                        }
                    };
                    let light_status = LightStatus {
                        lane_id: lane.id,
                        status: status.to_string(),
                    };
                    // Publish to the "light_status" exchange.
                    publish_message(&channel_clone, "light_status", "", &light_status).await;
                }
                // Log the current phase.
                let log_event = LogEvent {
                    source: format!("Junction-{}", junction),
                    message: format!("Phase {} active: Green lanes {:?}, Red lanes {:?}", group_index, green_lanes, red_lanes),
                    timestamp: current_time_secs(),
                };
                let _ = publish_message(&channel_clone, "logs", "", &log_event).await;
                // Green phase: hold for 5 seconds.
                sleep(Duration::from_secs(5)).await;
                // All-red clearance phase.
                {
                    let mut lights = tl_clone.lock().await;
                    for lane in &lane_list {
                        lights.insert(lane.id, LightColor::Red);
                    }
                }
                // Publish the all-red status.
                for lane in &lane_list {
                    let light_status = LightStatus {
                        lane_id: lane.id,
                        status: "Red".to_string(),
                    };
                    publish_message(&channel_clone, "light_status", "", &light_status).await;
                }
                sleep(Duration::from_secs(10)).await;
                // Move to the next group.
                group_index = (group_index + 1) % groups.len();
            }
        });
    }

    // Separately, subscribe to recommendations from RabbitMQ.
    let queue = channel.queue_declare("", lapin::options::QueueDeclareOptions::default(), lapin::types::FieldTable::default()).await?;
    channel.queue_bind(queue.name().as_str(), "recommendations", "", lapin::options::QueueBindOptions::default(), lapin::types::FieldTable::default()).await?;
    let mut consumer = channel.basic_consume(queue.name().as_str(), "traffic_light_recs", lapin::options::BasicConsumeOptions::default(), lapin::types::FieldTable::default()).await?;
    
    println!("Traffic Light Controller waiting for recommendations...");
    while let Some(delivery_result) = consumer.next().await {
        if let Ok(delivery) = delivery_result {
            let data = delivery.data.clone();
            if let Ok(rec) = serde_json::from_slice::<Recommendation>(&data) {
                println!("Received recommendation: {:?}", rec);
                let mut lights = traffic_lights.lock().await;
                if let Some(light) = lights.get_mut(&rec.lane_id) {
                    *light = LightColor::Green;
                    let log_event = LogEvent {
                        source: format!("TrafficLight-{}", rec.lane_id),
                        message: "Set to Green per recommendation".into(),
                        timestamp: current_time_secs(),
                    };
                    let _ = publish_message(&channel, "logs", "", &log_event).await;
                }
            }
            delivery.ack(lapin::options::BasicAckOptions::default()).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_traffic_lights().await {
        eprintln!("Error in traffic light controller: {}", e);
    }
}
