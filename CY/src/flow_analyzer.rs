use tokio;
use lapin::{options::*, types::FieldTable, Channel, ExchangeKind};
use futures_util::stream::StreamExt;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

mod mq;
use mq::{create_channel, publish_message, declare_exchange};
mod lanes;
use lanes::{load_lanes, Lane, LaneCategory};

/// --- Configuration Constants ---
const ACCELERATION: f64 = 300.0; // Simulation acceleration factor.
const HISTORY_WINDOW: u64 = 1500;  // 25 minutes in seconds.

/// --- Data Structures for Messaging ---
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrafficUpdate {
    pub lane_id: u32,
    pub vehicle_count: u32,
    pub average_waiting_time: f64,
    pub cars: Vec<CarWaitInfo>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarWaitInfo {
    pub car_id: u32,
    pub waiting_time: f64,
}

/// Modified Recommendation struct carries lane_id and a change_level (priority level).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Recommendation {
    pub lane_id: u32,
    pub change_level: u32,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

/// Helper: Return current time in seconds since Unix epoch.
fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// --- JSON Logging Structures ---
/// This enum is used for lane-specific or generic log events.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
enum LogEntry {
    LaneLog(FlowAnalyzerLog),
    GenericLog(LogEvent),
}

/// Log structure for lane-specific events.
#[derive(Serialize, Deserialize, Debug)]
struct FlowAnalyzerLog {
    lane_id: u32,
    recent_history: Vec<TrafficUpdate>,
    waiting_times: Vec<f64>,
    recommendation: Option<Recommendation>,
    timestamp: u64,
}

/// New structure for logging just the average_waiting_time from each TrafficUpdate.
#[derive(Serialize, Deserialize, Debug)]
struct MessageLogEntry {
    lane_id: u32,
    average_waiting_time: f64,
    timestamp: u64,
}

/// Write a JSON log entry (for lane logs or generic logs) to "flow_analyzer_log.json".
async fn write_json_log(entry: LogEntry) {
    let file_path = "flow_analyzer_log.json";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await
        .expect("Unable to open log file");
    let json_line = serde_json::to_string(&entry).expect("Failed to serialize log entry");
    file.write_all((json_line + "\n").as_bytes())
        .await
        .expect("Failed to write log entry to file");
}

/// Write a MessageLogEntry (containing average_waiting_time) to "message_log.json".
async fn write_message_log(entry: MessageLogEntry) {
    let file_path = "message_log.json";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await
        .expect("Unable to open message log file");
    let json_line = serde_json::to_string(&entry).expect("Failed to serialize message log entry");
    file.write_all((json_line + "\n").as_bytes())
        .await
        .expect("Failed to write message log entry to file");
}

/// --- Internal Structure for Analysis ---
/// IntersectionState holds the grouping information as well as a history of updates for each lane.
#[derive(Debug, Clone)]
struct IntersectionState {
    phases: Vec<Vec<u32>>,                 // Groups of lane IDs forming a phase.
    lane_to_phase: HashMap<u32, usize>,      // Mapping from lane ID to phase index.
    history: HashMap<u32, Vec<TrafficUpdate>>, // History of updates for each lane.
    last_recommendation: HashMap<u32, u64>,    // Last recommendation timestamp for each lane.
}

impl IntersectionState {
    fn new(intersection_id: u32, all_lanes: &[Lane]) -> Self {
        let junction_lanes: Vec<Lane> = all_lanes
            .iter()
            .filter(|l| l.end_intersection == intersection_id && l.category != LaneCategory::OutputBoundary)
            .cloned()
            .collect();

        let phases = group_lanes_by_direction(junction_lanes.as_slice());

        let mut lane_to_phase = HashMap::new();
        for (phase_index, lane_ids) in phases.iter().enumerate() {
            for lane_id in lane_ids {
                lane_to_phase.insert(*lane_id, phase_index);
            }
        }

        // Initialize history for each lane in the junction.
        let mut history = HashMap::new();
        let mut last_recommendation = HashMap::new();
        for lane in junction_lanes {
            history.insert(lane.id, Vec::new());
            last_recommendation.insert(lane.id, 0);
        }

        IntersectionState {
            phases,
            lane_to_phase,
            history,
            last_recommendation,
        }
    }

    /// Process a new update:
    /// - Append it to the lane's history.
    /// - Purge entries older than HISTORY_WINDOW.
    /// - Compute the average waiting time for this lane and for the other lanes.
    /// - If the ratio exceeds a threshold and no recent recommendation has been given, return a Recommendation.
    ///
    /// The recommendation cooldown is set to 300 simulation seconds,
    /// which corresponds to (300 / ACCELERATION) real seconds.
    fn process_update(&mut self, update: TrafficUpdate) -> Option<Recommendation> {
        let now = current_time_secs();
        if let Some(hist) = self.history.get_mut(&update.lane_id) {
            hist.push(update.clone());
            hist.retain(|upd| now.saturating_sub(upd.timestamp) <= HISTORY_WINDOW);
        }
        let lane_avg = if let Some(hist) = self.history.get(&update.lane_id) {
            if !hist.is_empty() {
                let sum: f64 = hist.iter().map(|u| u.average_waiting_time).sum();
                sum / (hist.len() as f64)
            } else {
                update.average_waiting_time
            }
        } else {
            update.average_waiting_time
        };

        let mut total = 0.0;
        let mut count = 0;
        for (&lane_id, hist) in &self.history {
            if lane_id != update.lane_id && !hist.is_empty() {
                let avg: f64 = hist.iter().map(|u| u.average_waiting_time).sum::<f64>() / (hist.len() as f64);
                total += avg;
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        let others_avg = total / (count as f64);
        if others_avg == 0.0 {
            return None;
        }
        let ratio = lane_avg / others_avg;
        // Check if a recommendation was issued for this lane within the last 300 simulation seconds.
        // In real time, this interval is (300 / ACCELERATION) seconds.
        let last_rec = *self.last_recommendation.get(&update.lane_id).unwrap_or(&0);
        if now < last_rec + (300.0 / ACCELERATION) as u64 {
            return None;
        }
        if ratio >= 1.4 {
            let change_level = if ratio >= 1.8 {
                3
            } else if ratio >= 1.6 {
                2
            } else {
                1
            };
            // Update the last recommendation timestamp.
            self.last_recommendation.insert(update.lane_id, now);
            Some(Recommendation {
                lane_id: update.lane_id,
                change_level,
                timestamp: now,
            })
        } else {
            None
        }
    }
}

/// --- Lane Grouping Logic ---
/// This grouping function mirrors that of traffic_light.rs using a threshold of 20 degrees.
fn intersection_to_coords(inter: u32) -> (f64, f64) {
    let row = ((inter - 1) / 4) as f64;
    let col = ((inter - 1) % 4) as f64;
    (row, col)
}

fn compute_lane_angle(lane: &Lane) -> f64 {
    if lane.start_intersection != 0 && lane.end_intersection != 0 {
        let (sx, sy) = intersection_to_coords(lane.start_intersection);
        let (ex, ey) = intersection_to_coords(lane.end_intersection);
        let dx = ex - sx;
        let dy = ey - sy;
        dy.atan2(dx).to_degrees().rem_euclid(360.0)
    } else {
        if lane.start_intersection == 0 {
            let (ex, ey) = intersection_to_coords(lane.end_intersection);
            if ex == 0.0 { 180.0 } else if ex == 3.0 { 0.0 } else if ey == 0.0 { 270.0 } else { 90.0 }
        } else { 0.0 }
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
                group.0 = (group.0 * ((group.1.len() as f64) - 1.0) + angle) / (group.1.len() as f64);
                added = true;
                break;
            }
        }
        if !added {
            groups.push((angle, vec![lane.id]));
        }
    }
    groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    groups.into_iter().map(|(_, ids)| ids).collect()
}

/// --- Main Analyzer Logic ---
pub async fn run_flow_analyzer() -> Result<(), Box<dyn std::error::Error>> {
    let channel = create_channel().await;
    declare_exchange(&channel, "simulation.updates", ExchangeKind::Fanout).await;
    declare_exchange(&channel, "recommendations", ExchangeKind::Fanout).await;
    declare_exchange(&channel, "logs", ExchangeKind::Fanout).await;

    // Set up the consumer queue for simulation updates.
    let queue = channel
        .queue_declare(
            "flow_analyzer_queue",
            QueueDeclareOptions {
                exclusive: false,
                durable: false,
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    channel
        .queue_bind(
            queue.name().as_str(),
            "simulation.updates",
            "",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            queue.name().as_str(),
            "flow_analyzer_consumer",
            BasicConsumeOptions {
                no_ack: false,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    println!("Flow Analyzer waiting for simulation updates...");

    let all_lanes = load_lanes();
    let mut intersection_states_map: HashMap<u32, IntersectionState> = HashMap::new();
    let mut lane_to_intersection_map: HashMap<u32, u32> = HashMap::new();

    // Map lanes to intersections.
    for lane in &all_lanes {
        if lane.end_intersection != 0 && lane.category != LaneCategory::OutputBoundary {
            lane_to_intersection_map.insert(lane.id, lane.end_intersection);
            intersection_states_map
                .entry(lane.end_intersection)
                .or_insert_with(|| IntersectionState::new(lane.end_intersection, all_lanes.as_slice()));
        }
    }
    let global_intersection_states = Arc::new(Mutex::new(intersection_states_map));

    // Consumer loop: process simulation updates.
    while let Some(delivery_result) = consumer.next().await {
        match delivery_result {
            Ok(delivery) => {
                let data = delivery.data.clone();
                match serde_json::from_slice::<TrafficUpdate>(&data) {
                    Ok(update) => {
                        // Write the average_waiting_time from the update to a separate message log file.
                        let msg_entry = MessageLogEntry {
                            lane_id: update.lane_id,
                            average_waiting_time: update.average_waiting_time,
                            timestamp: update.timestamp,
                        };
                        tokio::spawn(write_message_log(msg_entry));

                        if let Some(&intersection_id) = lane_to_intersection_map.get(&update.lane_id) {
                            let mut states = global_intersection_states.lock().await;
                            if let Some(state) = states.get_mut(&intersection_id) {
                                // Process the update and possibly generate a recommendation.
                                if let Some(rec) = state.process_update(update.clone()) {
                                    println!("Recommendation: Lane {} requires priority adjustment, Priority Level: {}", rec.lane_id, rec.change_level);
                                    publish_message(&channel, "recommendations", "", &rec).await;
                                    let log = LogEvent {
                                        source: "FlowAnalyzer".into(),
                                        message: format!("Published recommendation for lane {} with change level {}", rec.lane_id, rec.change_level),
                                        timestamp: current_time_secs(),
                                    };
                                    publish_message(&channel, "logs", "", &log).await;
                                }
                            } else {
                                eprintln!("Error: Intersection state not found for ID {}", intersection_id);
                            }
                        }
                        if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                            eprintln!("Failed to acknowledge message: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to deserialize TrafficUpdate: {}", e);
                        if let Err(e) = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await {
                            eprintln!("Failed to NACK message: {}", e);
                        }
                    }
                }
            },
            Err(e) => {
                eprintln!("Error receiving message from RabbitMQ: {}", e);
                break;
            }
        }
    }
    println!("Flow Analyzer stopped.");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_flow_analyzer().await {
        eprintln!("Error in flow analyzer: {}", e);
    }
}
