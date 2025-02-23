// traffic_light.rs
use tokio;
use lapin::{options::*, types::FieldTable};
use futures_util::stream::StreamExt;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

mod mq;
use mq::{create_channel, declare_exchange, publish_message};

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

fn current_time_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Initialize the traffic lights mapping. (For demo purposes, we assume lane IDs 2000–2009 are controlled.)
pub fn initialize_traffic_lights() -> HashMap<u32, LightColor> {
    let mut map = HashMap::new();
    for lane_id in 2000..2010 {
        map.insert(lane_id, LightColor::Red);
    }
    map
}

#[tokio::main]
async fn main() {
    let channel = create_channel().await;
    declare_exchange(&channel, "recommendations", lapin::ExchangeKind::Fanout).await;
    declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;

    // Create a temporary queue and bind it to the recommendations exchange.
    let queue = channel.queue_declare("", QueueDeclareOptions::default(), FieldTable::default())
        .await.expect("Queue declare failed");
    channel.queue_bind(queue.name().as_str(), "recommendations", "", QueueBindOptions::default(), FieldTable::default())
        .await.expect("Queue bind failed");

    let mut consumer = channel.basic_consume(queue.name().as_str(), "traffic_light", BasicConsumeOptions::default(), FieldTable::default())
        .await.expect("Failed to create consumer");

    // Initialize local traffic lights state.
    let mut traffic_lights = initialize_traffic_lights();

    // Spawn a periodic task that cycles local traffic lights.
    let channel_for_cycle = channel.clone();
    tokio::spawn(async move {
        loop {
            for (lane, color) in traffic_lights.iter_mut() {
                *color = if *color == LightColor::Red { LightColor::Green } else { LightColor::Red };
                let log = LogEvent {
                    source: format!("TrafficLight-{}", lane),
                    message: format!("Cycled to {:?}", color),
                    timestamp: current_time_secs(),
                };
                publish_message(&channel_for_cycle, "logs", "", &log).await;
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    println!("Traffic Light Controller waiting for recommendations...");

    while let Some(delivery_result) = consumer.next().await {
        if let Ok(delivery) = delivery_result {
            let data = delivery.data;
            if let Ok(rec) = serde_json::from_slice::<Recommendation>(&data) {
                println!("Received recommendation: {:?}", rec);
                if let Some(light) = traffic_lights.get_mut(&rec.lane_id) {
                    *light = LightColor::Green;
                    let log = LogEvent {
                        source: format!("TrafficLight-{}", rec.lane_id),
                        message: "Set to Green per recommendation".into(),
                        timestamp: current_time_secs(),
                    };
                    publish_message(&channel, "logs", "", &log).await;
                }
            }
            delivery.ack(BasicAckOptions::default()).await.expect("Ack failed");
        }
    }
}
