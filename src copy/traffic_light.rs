use tokio::time::{sleep, Duration};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use futures_util::stream::StreamExt;
use tokio::fs::OpenOptions;
use serde_json;
use tokio::io::AsyncWriteExt;
mod mq;
use mq::{create_channel, declare_exchange, publish_message};
mod lanes;
use lanes::{load_lanes, Lane};
use tokio;
use lapin::ExchangeKind;
use rand::Rng;
use std::error::Error;

mod model;
use model::{LightStatus, SimulationConfig, load_simulation_config};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LightColor {
    Red,
    Green,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Recommendation {
    pub lane_id: u32,
    pub change_level: u32, // 1, 2, or 3
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn intersection_to_coords(inter: u32) -> (f64, f64) {
    let row = ((inter - 1) / 4) as f64;
    let col = ((inter - 1) % 4) as f64;
    (row, col)
}

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
        let (ex, _) = intersection_to_coords(lane.end_intersection);
        if ex == 0.0 {
            90.0
        } else if ex == 3.0 {
            270.0
        } else {
            90.0
        }
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

pub type TrafficLightMap = Arc<Mutex<HashMap<u32, LightColor>>>;

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

#[derive(Debug)]
struct JunctionPhases {
    groups: Vec<Vec<u32>>,
    timings: Vec<f64>,
}

#[derive(Serialize)]
struct JunctionTimingLog {
    junction_id: u32,
    groups: Vec<Vec<u32>>,
    timings: Vec<f64>,
    timestamp: u64,
}

pub async fn run_traffic_lights() -> Result<(), Box<dyn Error>> {
    let channel = create_channel().await;
    declare_exchange(&channel, "logs", ExchangeKind::Fanout).await;
    declare_exchange(&channel, "recommendations", ExchangeKind::Fanout).await;
    declare_exchange(&channel, "light_status", ExchangeKind::Fanout).await;

    let traffic_lights = initialize_traffic_lights();

    let lanes = load_lanes();
    let mut junction_map: HashMap<u32, Vec<Lane>> = HashMap::new();
    for lane in lanes {
        if lane.end_intersection != 0 {
            junction_map.entry(lane.end_intersection).or_default().push(lane);
        }
    }
    
    let junction_phases_map: Arc<Mutex<HashMap<u32, Arc<Mutex<JunctionPhases>>>>> = Arc::new(Mutex::new(HashMap::new()));
    let lane_phase_mapping: Arc<Mutex<HashMap<u32, (u32, usize)>>> = Arc::new(Mutex::new(HashMap::new()));

    // Load simulation configuration from model (base timing value and acceleration)
    let sim_config: SimulationConfig = load_simulation_config();
    let acceleration = sim_config.acceleration;
    
    for (junction, lane_list) in junction_map.iter() {
        let groups = group_lanes_by_direction(lane_list.as_slice());
        let default_timing = sim_config.base_timing;
        let timings = vec![default_timing; groups.len()];
        let jp = Arc::new(Mutex::new(JunctionPhases {
            groups: groups.clone(),
            timings,
        }));
        {
            let mut mapping = lane_phase_mapping.lock().await;
            for (phase_index, lane_ids) in groups.iter().enumerate() {
                for lane_id in lane_ids {
                    mapping.insert(*lane_id, (*junction, phase_index));
                }
            }
        }
        {
            let mut jmap = junction_phases_map.lock().await;
            jmap.insert(*junction, Arc::clone(&jp));
        }
        let tl_clone = Arc::clone(&traffic_lights);
        let channel_clone = channel.clone();
        let lane_list_clone = lane_list.clone();
        let junction_id = *junction;
        let jp_clone = Arc::clone(&jp);
        tokio::spawn(async move {
            let mut group_index = 0;
            loop {
                let current_phase = {
                    let jp_lock = jp_clone.lock().await;
                    jp_lock.groups[group_index].clone()
                };
                let mut green_lanes = Vec::new();
                let mut red_lanes = Vec::new();
                {
                    let mut lights = tl_clone.lock().await;
                    for lane in &lane_list_clone {
                        if current_phase.contains(&lane.id) {
                            lights.insert(lane.id, LightColor::Green);
                            green_lanes.push(lane.id);
                        } else {
                            lights.insert(lane.id, LightColor::Red);
                            red_lanes.push(lane.id);
                        }
                    }
                }
                for lane in &lane_list_clone {
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
                    publish_message(&channel_clone, "light_status", "", &light_status).await;
                }
                let log_event = LogEvent {
                    source: format!("Junction-{}", junction_id),
                    message: format!("Phase {} active: Green lanes {:?}, Red lanes {:?}", group_index, green_lanes, red_lanes),
                    timestamp: current_time_secs(),
                };
                let _ = publish_message(&channel_clone, "logs", "", &log_event).await;
                let phase_timing = {
                    let jp_lock = jp_clone.lock().await;
                    jp_lock.timings[group_index]
                };
                sleep(Duration::from_secs_f64(phase_timing / acceleration)).await;
                group_index = (group_index + 1) % {
                    let jp_lock = jp_clone.lock().await;
                    jp_lock.groups.len()
                };
            }
        });
    }

    {
        let junction_phases_map_clone = Arc::clone(&junction_phases_map);
        tokio::spawn(async move {
            let mut prev_snapshot: Option<Vec<(u32, Vec<Vec<u32>>, Vec<f64>)>> = None;
            loop {
                sleep(Duration::from_secs_f64(120.0 / acceleration)).await;
                let mut current_snapshot: Vec<(u32, Vec<Vec<u32>>, Vec<f64>)> = Vec::new();
                {
                    let jmap = junction_phases_map_clone.lock().await;
                    for (junction_id, jp_arc) in jmap.iter() {
                        let jp = jp_arc.lock().await;
                        current_snapshot.push((*junction_id, jp.groups.clone(), jp.timings.clone()));
                    }
                }
                if let Some(prev) = &prev_snapshot {
                    if *prev == current_snapshot {
                        continue;
                    }
                }
                prev_snapshot = Some(current_snapshot.clone());
                let log_events: Vec<JunctionTimingLog> = current_snapshot.into_iter().map(|(junction_id, groups, timings)| {
                    JunctionTimingLog {
                        junction_id,
                        groups,
                        timings,
                        timestamp: current_time_secs(),
                    }
                }).collect();
                let json_line = serde_json::to_string(&log_events).unwrap();
                let file_path = "light_timings_log.json";
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(file_path).await {
                    let _ = file.write_all((json_line + "\n").as_bytes()).await;
                }
            }
        });
    }

    let lane_phase_mapping_clone = Arc::clone(&lane_phase_mapping);
    let junction_phases_map_clone = Arc::clone(&junction_phases_map);
    let channel_clone = channel.clone();
    tokio::spawn(async move {
        let queue = channel.queue_declare("", lapin::options::QueueDeclareOptions::default(), lapin::types::FieldTable::default()).await.unwrap();
        channel.queue_bind(queue.name().as_str(), "recommendations", "", lapin::options::QueueBindOptions::default(), lapin::types::FieldTable::default()).await.unwrap();
        let mut consumer = channel.basic_consume(queue.name().as_str(), "traffic_light_recs", lapin::options::BasicConsumeOptions::default(), lapin::types::FieldTable::default()).await.unwrap();
        println!("Traffic Light Controller waiting for recommendations...");
        while let Some(delivery_result) = consumer.next().await {
            if let Ok(delivery) = delivery_result {
                let data = delivery.data.clone();
                if let Ok(rec) = serde_json::from_slice::<Recommendation>(&data) {
                    println!("Received recommendation: {:?}", rec);
                    let mapping = lane_phase_mapping_clone.lock().await;
                    if let Some(&(junction_id, phase_index)) = mapping.get(&rec.lane_id) {
                        drop(mapping);
                        let jmap = junction_phases_map_clone.lock().await;
                        if let Some(jp_arc) = jmap.get(&junction_id) {
                            let mut jp_lock = jp_arc.lock().await;
                            let add_time = match rec.change_level {
                                1 => 2.0,
                                2 => 4.0,
                                3 => 6.0,
                                _ => 0.0,
                            };
                            if jp_lock.timings.len() == 3 {
                                let orig_sum: f64 = jp_lock.timings.iter().sum();
                                let target_old = jp_lock.timings[phase_index];
                                let new_target = if target_old - 2.0 < 15.0 { 15.0 } else { target_old - 2.0 };
                                let mut new_timings = jp_lock.timings.clone();
                                new_timings[phase_index] = new_target;
                                for i in 0..3 {
                                    if i != phase_index {
                                        if jp_lock.timings[i] <= 15.0 {
                                            new_timings[i] = 15.0;
                                        } else {
                                            new_timings[i] = jp_lock.timings[i] + 1.0;
                                        }
                                    }
                                }
                                let new_sum: f64 = new_timings.iter().sum();
                                let error = orig_sum - new_sum;
                                for i in 0..3 {
                                    if i != phase_index && new_timings[i] > 15.0 {
                                        new_timings[i] += error;
                                        break;
                                    }
                                }
                                jp_lock.timings = new_timings;
                                let log_event = LogEvent {
                                    source: format!("TrafficLight-{}", rec.lane_id),
                                    message: format!("Special adjusted timings at Junction {}: {:?}", junction_id, jp_lock.timings),
                                    timestamp: current_time_secs(),
                                };
                                let _ = publish_message(&channel_clone, "logs", "", &log_event).await;
                            } else {
                                let mut total_available = 0.0;
                                for (i, timing) in jp_lock.timings.iter().enumerate() {
                                    if i != phase_index {
                                        let available = if *timing > 15.0 { *timing - 15.0 } else { 0.0 };
                                        total_available += available;
                                    }
                                }
                                if total_available < add_time {
                                    let log_event = LogEvent {
                                        source: format!("TrafficLight-{}", rec.lane_id),
                                        message: format!("Skipped timing adjustment at Junction {} due to insufficient available time (required {:.2}s, available {:.2}s)", junction_id, add_time, total_available),
                                        timestamp: current_time_secs(),
                                    };
                                    let _ = publish_message(&channel_clone, "logs", "", &log_event).await;
                                } else {
                                    for (i, timing) in jp_lock.timings.iter_mut().enumerate() {
                                        if i == phase_index {
                                            *timing += add_time;
                                        } else {
                                            let available = if *timing > 15.0 { *timing - 15.0 } else { 0.0 };
                                            let subtract = (available / total_available) * add_time;
                                            *timing = (*timing - subtract).max(15.0);
                                        }
                                    }
                                    let log_event = LogEvent {
                                        source: format!("TrafficLight-{}", rec.lane_id),
                                        message: format!("Adjusted timings at Junction {}: Added {:.2}s to phase {} and subtracted proportionally from others", junction_id, add_time, phase_index),
                                        timestamp: current_time_secs(),
                                    };
                                    let _ = publish_message(&channel_clone, "logs", "", &log_event).await;
                                }
                            }
                        }
                    } else {
                        println!("Lane {} not found in phase mapping", rec.lane_id);
                    }
                }
                delivery.ack(lapin::options::BasicAckOptions::default()).await.unwrap();
            }
        }
    });

    // Load simulation configuration to obtain acceleration value.
    let sim_config = load_simulation_config();
    let acceleration = sim_config.acceleration;

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_traffic_lights().await {
        eprintln!("Error in traffic light controller: {}", e);
    }
}
