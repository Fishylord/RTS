use tokio;
use std::sync::Arc;
use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::cmp::Ordering;
use tokio::time::{sleep, Duration, Instant};
use serde::{Serialize, Deserialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use dashmap::DashMap;
use tokio::sync::watch;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};

mod mq;
mod lanes; // lanes.rs must be in the same folder
use lanes::{load_lanes, Lane, LaneCategory};

mod model;
use model::LightStatus;

// -----------------------------------------------------------------------------
// Revised FIFO Queue for a Lane with Forced Check
// -----------------------------------------------------------------------------
use tokio::sync::{Mutex as AsyncMutex, oneshot};

#[derive(Clone)]
pub struct LaneQueue {
    busy: Arc<AsyncMutex<bool>>,
    // Each entry: (oneshot sender, car_id, enqueued time, car data)
    queue: Arc<AsyncMutex<VecDeque<(oneshot::Sender<()>, u32, Instant, String)>>>,
}

impl LaneQueue {
    pub fn new() -> Self {
        Self {
            busy: Arc::new(AsyncMutex::new(false)),
            queue: Arc::new(AsyncMutex::new(VecDeque::new())),
        }
    }

    pub async fn wait_in_queue<F>(&self, car_id: u32, lane_id: u32, car_data: String, get_light_status: F)
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        println!("Car {}: Attempting to enter queue for lane {}", car_id, lane_id);
        let mut busy_guard = self.busy.lock().await;
        if !*busy_guard {
            println!("Car {}: Lane {} is free, proceeding immediately", car_id, lane_id);
            *busy_guard = true;
            return;
        }
        drop(busy_guard);
        let (tx, rx) = oneshot::channel();
        let enqueued_at = Instant::now();
        {
            let mut q = self.queue.lock().await;
            q.push_back((tx, car_id, enqueued_at, car_data.clone()));
            println!("Car {}: Enqueued in lane {} queue", car_id, lane_id);
        }
        let check_interval = Duration::from_secs(1);
        let queue_clone = Arc::clone(&self.queue);
        let get_light_status = Arc::new(get_light_status);
        let get_light_status_clone = get_light_status.clone();
        tokio::spawn(async move {
            loop {
                sleep(check_interval).await;
                let mut q = queue_clone.lock().await;
                if q.len() == 1 {
                    let (sender, front_car_id, _enq_time, _front_car_data) = q.pop_front().unwrap();
                    if front_car_id == car_id {
                        let light_status = get_light_status_clone();
                        if light_status == "Green" {
                            println!("Car {}: Alone in queue and light is green. Forcing turn.", car_id);
                            let _ = sender.send(());
                            break;
                        } else {
                            q.push_front((sender, front_car_id, Instant::now(), String::new()));
                        }
                    } else {
                        break;
                    }
                } else if q.is_empty() {
                    break;
                }
            }
        });
        let _ = rx.await;
        println!("Car {}: Woken up from queue for lane {}", car_id, lane_id);
        let mut busy_guard = self.busy.lock().await;
        *busy_guard = true;
    }

    pub async fn release(&self, car_id: u32, lane_id: u32) {
        let mut q = self.queue.lock().await;
        if let Some((tx, next_car_id, _, _)) = q.pop_front() {
            println!("Car {}: Releasing lane {} queue to next car (car {}).", car_id, lane_id, next_car_id);
            let _ = tx.send(());
        } else {
            let mut busy_guard = self.busy.lock().await;
            println!("Car {}: No waiting cars in lane {}; marking lane free", car_id, lane_id);
            *busy_guard = false;
        }
    }
}

// -----------------------------------------------------------------------------
// Configuration & Constants
// -----------------------------------------------------------------------------

const ACCELERATION: f64 = 300.0; // simulated seconds per real second
const TURN_DELAY: f64 = 8.0;     // default turn delay in simulated seconds

struct SpawnInterval {
    start: f64,
    end: f64,
    count: u32,
    label: &'static str,
}

// -----------------------------------------------------------------------------
// Data Structures for Messaging and Metrics
// -----------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct TrafficUpdate {
    pub lane_id: u32,
    pub vehicle_count: u32,
    pub average_waiting_time: f64,
    pub cars: Vec<CarWaitInfo>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct CarWaitInfo {
    pub car_id: u32,
    pub waiting_time: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarRouteData {
    pub car_id: u32,
    pub speed: f64,
    pub spawn_category: String,
    pub route: Vec<u32>,
    pub lanes: Vec<LaneData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LaneData {
    pub lane_id: u32,
    pub waiting_time: f64,
    pub drive_time: f64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CarMetrics {
    pub id: u32,
    pub wait_time: f64,
    pub drive_time: f64,
    pub total_time: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CarLog {
    pub car_route: CarRouteData,
    pub metrics: CarMetrics,
}

// -----------------------------------------------------------------------------
// Utility Functions
// -----------------------------------------------------------------------------

fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

// -----------------------------------------------------------------------------
// Dijkstra’s Algorithm for Shortest Lane Path
// -----------------------------------------------------------------------------

fn find_shortest_path(start: u32, end: u32, lanes: &[Lane]) -> Option<Vec<Lane>> {
    #[derive(Debug)]
    struct LaneState {
        cost: f64,
        position: u32,
    }
    impl Eq for LaneState {}
    impl PartialEq for LaneState {
        fn eq(&self, other: &Self) -> bool { self.cost.eq(&other.cost) }
    }
    impl Ord for LaneState {
        fn cmp(&self, other: &Self) -> Ordering {
            other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
        }
    }
    impl PartialOrd for LaneState {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
    }

    let mut dist: HashMap<u32, f64> = HashMap::new();
    let mut prev: HashMap<u32, (u32, Lane)> = HashMap::new();
    let mut heap = BinaryHeap::new();

    for inter in 1..=16 { dist.insert(inter, std::f64::INFINITY); }
    dist.insert(start, 0.0);
    heap.push(LaneState { cost: 0.0, position: start });

    let mut lane_map: HashMap<u32, Vec<&Lane>> = HashMap::new();
    for lane in lanes { lane_map.entry(lane.start_intersection).or_default().push(lane); }

    while let Some(LaneState { cost, position }) = heap.pop() {
        if position == end { break; }
        if cost > dist[&position] { continue; }
        if let Some(neighbors) = lane_map.get(&position) {
            for &lane in neighbors {
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

    if !dist.contains_key(&end) || dist[&end] == std::f64::INFINITY { return None; }

    let mut path = Vec::new();
    let mut current = end;
    while current != start {
        if let Some(&(prev_inter, ref lane)) = prev.get(&current) {
            path.push(lane.clone());
            current = prev_inter;
        } else { break; }
    }
    path.reverse();
    Some(path)
}

// -----------------------------------------------------------------------------
// Shared Simulation State
// -----------------------------------------------------------------------------

pub type SimEvent = Arc<DashMap<u32, u32>>;
pub fn initialize_simdata() -> SimEvent {
    let lanes = load_lanes();
    let map = DashMap::new();
    for lane in lanes { map.insert(lane.id, 0); }
    Arc::new(map)
}

pub struct LightStatusChannel {
    pub sender: watch::Sender<String>,
    pub receiver: watch::Receiver<String>,
}
pub type LightStatusMap = Arc<DashMap<u32, LightStatusChannel>>;

pub type LaneQueues = Arc<HashMap<u32, Arc<LaneQueue>>>;
pub type LaneWaitTimes = Arc<DashMap<u32, DashMap<u32, f64>>>;

// -----------------------------------------------------------------------------
// Light Status Listener
// -----------------------------------------------------------------------------

async fn listen_for_light_statuses(channel: &lapin::Channel, light_status_map: LightStatusMap)
    -> Result<(), Box<dyn std::error::Error>>
{
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
            if let Some(entry) = light_status_map.get(&light_status.lane_id) {
                let _ = entry.sender.send(light_status.status.clone());
                println!("Updated light status for lane {}: {:?}", light_status.lane_id, light_status.status);
            }
        }
        delivery.ack(lapin::options::BasicAckOptions::default()).await?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Spawn Schedule Generation
// -----------------------------------------------------------------------------

fn generate_spawn_schedule() -> Vec<(f64, u32, &'static str)> {
    let intervals = vec![
        SpawnInterval { start: 0.0, end: 2.0 * 3600.0, count: 500, label: "Low" },
        SpawnInterval { start: 2.0 * 3600.0, end: 4.0 * 3600.0, count: 5000, label: "Heavy" },
        SpawnInterval { start: 4.0 * 3600.0, end: 6.0 * 3600.0, count: 500, label: "Low" },
    ];

    let mut schedule = Vec::new();
    let mut car_id = 1;
    let mut rng = rand::thread_rng();
    for interval in intervals {
        for _ in 0..interval.count {
            let t = rng.gen_range(interval.start..interval.end);
            schedule.push((t, car_id, interval.label));
            car_id += 1;
        }
    }
    schedule.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    schedule.into_iter().enumerate().map(|(i, (t, _, label))| (t, (i+1) as u32, label)).collect()
}

// -----------------------------------------------------------------------------
// Car Simulation Function with Integrated FIFO and Light Waiting
// -----------------------------------------------------------------------------

async fn simulate_car(
    car_id: u32,
    spawn_category: &'static str,
    channel: &lapin::Channel,
    sim_event: SimEvent,
    light_status_map: LightStatusMap,
    lane_locks: Arc<HashMap<u32, Arc<tokio::sync::Mutex<()>>>>,
    lane_queues: LaneQueues,
    lane_wait_times: LaneWaitTimes,
    json_tx: mpsc::Sender<CarLog>,
) -> CarMetrics {
    let _start_time = Instant::now();
    let mut rng = ChaCha8Rng::seed_from_u64(42 + car_id as u64);
    let speed: f64 = rng.gen_range(3456.0..=7000.0);

    let all_lanes = load_lanes();
    let entry_lanes: Vec<Lane> = all_lanes.iter().filter(|l| l.category == LaneCategory::InputBoundary).cloned().collect();
    let exit_lanes: Vec<Lane> = all_lanes.iter().filter(|l| l.category == LaneCategory::OutputBoundary).cloned().collect();

    let input_lane = entry_lanes[rng.gen_range(0..entry_lanes.len())].clone();
    let mut exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    while exit_lane.id == input_lane.id {
        exit_lane = exit_lanes[rng.gen_range(0..exit_lanes.len())].clone();
    }

    let start_intersection = input_lane.end_intersection;
    let end_intersection = exit_lane.start_intersection;
    let internal_lanes: Vec<Lane> = all_lanes.into_iter().filter(|l| l.category == LaneCategory::Internal).collect();
    let lane_route = find_shortest_path(start_intersection, end_intersection, internal_lanes.as_slice()).unwrap_or_else(Vec::new);

    // Build full route: [input, internal lanes..., exit]
    let mut full_route_ids = Vec::new();
    full_route_ids.push(input_lane.id);
    let internal_ids: Vec<u32> = lane_route.iter().map(|lane| lane.id).collect();
    full_route_ids.extend(internal_ids);
    full_route_ids.push(exit_lane.id);

    let mut lane_data: Vec<LaneData> = Vec::new();
    let mut total_wait_time = 0.0;
    let mut total_drive_time = 0.0;

    // Process input lane (driving only; no waiting)
    if let Some(lock) = lane_locks.get(&input_lane.id) {
        let _guard = lock.lock().await;
        let seg = input_lane.length / speed;
        let seg_sim = seg * ACCELERATION;
        total_drive_time += seg_sim;
        println!("Car {}: Driving input lane {}", car_id, input_lane.id);
        sleep(Duration::from_secs_f64(seg / ACCELERATION)).await;
        lane_data.push(LaneData { lane_id: input_lane.id, waiting_time: 0.0, drive_time: seg_sim, timestamp: current_time_secs() });
    }

    // Process each internal lane.
    for (i, lane) in lane_route.iter().enumerate() {
        sim_event.entry(lane.id).and_modify(|v| *v += 1).or_insert(1);
        let seg = lane.length / speed;
        let seg_sim = seg * ACCELERATION;
        total_drive_time += seg_sim;
        println!("Car {}: Driving internal lane {}", car_id, lane.id);
        sleep(Duration::from_secs_f64(seg / ACCELERATION)).await;
        lane_data.push(LaneData { lane_id: lane.id, waiting_time: 0.0, drive_time: seg_sim, timestamp: current_time_secs() });
        // Start waiting (queue + light) for internal lanes.
        println!("Car {}: Starting wait on lane {}", car_id, lane.id);
        let wait_start = Instant::now();
        let car_data = format!("Car {} waiting in lane {}. Spawn category: {}, speed: {:.2}", car_id, lane.id, spawn_category, speed);
        let lane_id_for_closure = lane.id;
        let light_status_map_clone = light_status_map.clone();
        let get_light_status = move || {
            if let Some(light_channel) = light_status_map_clone.get(&lane_id_for_closure) {
                light_channel.receiver.borrow().clone()
            } else {
                "Unknown".to_string()
            }
        };
        if let Some(queue) = lane_queues.get(&lane.id) {
            queue.wait_in_queue(car_id, lane.id, car_data, get_light_status).await;
        }
        if let Some(light_channel) = light_status_map.get(&lane.id) {
            let mut rx = light_channel.receiver.clone();
            while rx.borrow().as_str() != "Green" {
                println!("Car {}: Waiting for lane {} light to be Green (current: {:?})", car_id, lane.id, rx.borrow());
                rx.changed().await.ok();
            }
            println!("Car {}: Detected lane {} light is Green", car_id, lane.id);
        }
        let lane_wait = wait_start.elapsed().as_secs_f64() * ACCELERATION;
        total_wait_time += lane_wait;
        lane_data.push(LaneData { lane_id: lane.id, waiting_time: lane_wait, drive_time: 0.0, timestamp: current_time_secs() });
        if let Some(wait_map) = lane_wait_times.get(&lane.id) {
            wait_map.insert(car_id, lane_wait);
        }
        if let Some(queue) = lane_queues.get(&lane.id) {
            queue.release(car_id, lane.id).await;
        }
        // For turn delay, only the sleep duration is adjusted; the recorded delay is already in simulated time.
        let turn_real = TURN_DELAY / ACCELERATION;
        let turn_sim = TURN_DELAY; // use TURN_DELAY directly (already simulated time)
        println!("Car {}: Executing turn delay on lane {} ({} simulated sec)", car_id, lane.id, TURN_DELAY);
        sleep(Duration::from_secs_f64(turn_real)).await;
        total_drive_time += turn_sim;
        let next_lane_id = lane_route.get(i + 1).map_or(lane.id, |next_lane| next_lane.id);
        lane_data.push(LaneData { lane_id: next_lane_id, waiting_time: 0.0, drive_time: turn_sim, timestamp: current_time_secs() });
        sim_event.entry(lane.id).and_modify(|v| *v -= 1);
        sleep(Duration::from_secs_f64(0.05 / ACCELERATION)).await;
    }

    // Process exit lane.
    println!("Car {}: Driving exit lane {} (no waiting)", car_id, exit_lane.id);
    let seg = exit_lane.length / speed;
    let seg_sim = seg * ACCELERATION;
    total_drive_time += seg_sim;
    sleep(Duration::from_secs_f64(seg / ACCELERATION)).await;
    lane_data.push(LaneData { lane_id: exit_lane.id, waiting_time: 0.0, drive_time: seg_sim, timestamp: current_time_secs() });

    let total_time = total_wait_time + total_drive_time;
    let mut consolidated_map: HashMap<u32, LaneData> = HashMap::new();
    for record in lane_data.into_iter() {
        consolidated_map.entry(record.lane_id).and_modify(|entry| {
            entry.waiting_time += record.waiting_time;
            entry.drive_time += record.drive_time;
        }).or_insert(record);
    }
    let mut unique_route: Vec<u32> = Vec::new();
    for &id in &full_route_ids {
        if !unique_route.contains(&id) { unique_route.push(id); }
    }
    let consolidated_lane_data: Vec<LaneData> = unique_route.iter().filter_map(|&id| consolidated_map.remove(&id)).collect();

    let car_route_data = CarRouteData {
        car_id,
        speed,
        spawn_category: spawn_category.to_string(),
        route: unique_route,
        lanes: consolidated_lane_data,
    };

    let comp_log = LogEvent {
        source: format!("Car-{}", car_id),
        message: format!("Completed journey: Wait={:.2}s, Drive={:.2}s, Total={:.2}s", total_wait_time, total_drive_time, total_time),
        timestamp: current_time_secs(),
    };
    mq::publish_message(channel, "logs", "", &comp_log).await;

    let final_log = CarLog {
        car_route: car_route_data,
        metrics: CarMetrics {
            id: car_id,
            wait_time: total_wait_time,
            drive_time: total_drive_time,
            total_time: total_time,
        },
    };
    if let Err(e) = json_tx.send(final_log).await {
        eprintln!("Failed to send final JSON log for car {}: {}", car_id, e);
    }
    CarMetrics { id: car_id, wait_time: total_wait_time, drive_time: total_drive_time, total_time }
}

// -----------------------------------------------------------------------------
// Main Simulation Entry Point with Asynchronous File I/O for Logging
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let channel = mq::create_channel().await;
    mq::declare_exchange(&channel, "simulation.updates", lapin::ExchangeKind::Fanout).await;
    mq::declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;
    mq::declare_exchange(&channel, "light_status", lapin::ExchangeKind::Fanout).await;

    let sim_event = initialize_simdata();
    let lanes = load_lanes();

    let light_status_map: LightStatusMap = Arc::new(DashMap::new());
    for lane in &lanes {
        let (sender, receiver) = watch::channel("Red".to_string());
        light_status_map.insert(lane.id, LightStatusChannel { sender, receiver });
    }

    let channel_clone = channel.clone();
    let light_status_map_clone = light_status_map.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_light_statuses(&channel_clone, light_status_map_clone).await {
            eprintln!("Error listening for light statuses: {}", e);
        }
    });

    let mut locks_map = HashMap::new();
    for lane in &lanes {
        locks_map.insert(lane.id, Arc::new(tokio::sync::Mutex::new(())));
    }
    let lane_locks = Arc::new(locks_map);

    let mut queue_map = HashMap::new();
    for lane in &lanes {
        queue_map.insert(lane.id, Arc::new(LaneQueue::new()));
    }
    let lane_queues: LaneQueues = Arc::new(queue_map);

    let lane_wait_times: LaneWaitTimes = Arc::new(DashMap::new());
    for lane in &lanes {
        lane_wait_times.insert(lane.id, DashMap::new());
    }

    let (json_tx, mut json_rx) = mpsc::channel::<CarLog>(100);
    tokio::spawn(async move {
        let file_path = "car_simulation_logs.json";
        let file = OpenOptions::new().create(true).append(true).open(file_path).await.expect("Unable to open log file");
        let mut writer = BufWriter::new(file);
        while let Some(car_log) = json_rx.recv().await {
            let json_str = serde_json::to_string(&car_log).expect("Failed to serialize car log");
            if let Err(e) = writer.write_all((json_str + "\n").as_bytes()).await {
                eprintln!("Failed to write to log file: {}", e);
            }
        }
    });

    let channel_stats = channel.clone();
    let sim_event_stats = sim_event.clone();
    let lane_wait_times_stats = lane_wait_times.clone();
    tokio::spawn(async move {
        loop {
            let mut stats_msgs = Vec::new();
            for entry in sim_event_stats.iter() {
                let lane_id = *entry.key();
                let vehicle_count = *entry.value();
                let (sum, count, cars) = if let Some(wait_map) = lane_wait_times_stats.get(&lane_id) {
                    let (s, c) = wait_map.iter().fold((0.0, 0), |(s, c), item| (s + *item.value(), c + 1));
                    let cars: Vec<CarWaitInfo> = wait_map.iter().map(|item| CarWaitInfo { car_id: *item.key(), waiting_time: *item.value() }).collect();
                    (s, c, cars)
                } else { (0.0, 0, Vec::new()) };
                let avg_wait = if count > 0 { sum / count as f64 } else { 0.0 };
                let lane_stat = TrafficUpdate { lane_id, vehicle_count, average_waiting_time: avg_wait, cars, timestamp: current_time_secs() };
                stats_msgs.push(lane_stat);
            }
            for stat in stats_msgs {
                mq::publish_message(&channel_stats, "simulation.updates", "", &stat).await;
            }
            sleep(Duration::from_secs_f64(60.0 / ACCELERATION)).await;
        }
    });

    let spawn_schedule = generate_spawn_schedule();
    let sim_start = Instant::now();
    let mut handles = Vec::new();
    for (spawn_time_sim, car_id, spawn_category) in spawn_schedule {
        let desired_wall_time = spawn_time_sim / ACCELERATION;
        let now = sim_start.elapsed().as_secs_f64();
        if desired_wall_time > now {
            sleep(Duration::from_secs_f64(desired_wall_time - now)).await;
        }
        let channel_clone = channel.clone();
        let sim_event_clone = sim_event.clone();
        let light_status_map_clone = light_status_map.clone();
        let lane_locks_clone = lane_locks.clone();
        let lane_queues_clone = lane_queues.clone();
        let lane_wait_times_clone = lane_wait_times.clone();
        let json_tx_clone = json_tx.clone();
        let handle = tokio::spawn(async move {
            simulate_car(car_id, spawn_category, &channel_clone, sim_event_clone, light_status_map_clone, lane_locks_clone, lane_queues_clone, lane_wait_times_clone, json_tx_clone).await
        });
        handles.push(handle);
    }

    let mut metrics_vec = Vec::new();
    for handle in handles {
        if let Ok(metrics) = handle.await {
            metrics_vec.push(metrics);
        }
    }
    let count = metrics_vec.len() as f64;
    let total_wait: f64 = metrics_vec.iter().map(|m| m.wait_time).sum();
    let total_drive: f64 = metrics_vec.iter().map(|m| m.drive_time).sum();
    let total_total: f64 = metrics_vec.iter().map(|m| m.total_time).sum();
    let avg_wait = total_wait / count;
    let avg_drive = total_drive / count;
    let avg_total = total_total / count;
    println!("Average Metrics: Wait={:.2}s, Drive={:.2}s, Total={:.2}s", avg_wait, avg_drive, avg_total);
    let avg_log = LogEvent { source: "Simulation".into(), message: format!("Average Metrics: Wait={:.2}s, Drive={:.2}s, Total={:.2}s", avg_wait, avg_drive, avg_total), timestamp: current_time_secs() };
    mq::publish_message(&channel, "logs", "", &avg_log).await;
    let complete_msg = LogEvent { source: "Simulation".into(), message: "SIMULATION_COMPLETE".into(), timestamp: current_time_secs() };
    mq::publish_message(&channel, "logs", "", &complete_msg).await;
}
