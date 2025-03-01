// flow_analyzer.rs
use tokio;
use lapin::{options::*, types::FieldTable};
use futures_util::stream::StreamExt;
use serde::{Serialize, Deserialize};

mod mq;
use mq::{create_channel, publish_message, declare_exchange};

#[derive(Serialize, Deserialize, Debug)]
pub struct TrafficUpdate {
    pub lane_id: u32,
    pub vehicle_count: u32,
    pub timestamp: u64,
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

pub async fn run_flow_analyzer() -> Result<(), Box<dyn std::error::Error>> {
    let channel = create_channel().await;
    declare_exchange(&channel, "simulation.updates", lapin::ExchangeKind::Fanout).await;
    declare_exchange(&channel, "recommendations", lapin::ExchangeKind::Fanout).await;
    declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;

    let queue = channel.queue_declare("", QueueDeclareOptions::default(), FieldTable::default())
        .await?;
    channel.queue_bind(queue.name().as_str(), "simulation.updates", "", QueueBindOptions::default(), FieldTable::default())
        .await?;

    let mut consumer = channel.basic_consume(queue.name().as_str(), "flow_analyzer", BasicConsumeOptions::default(), FieldTable::default())
        .await?;

    println!("Flow Analyzer waiting for simulation updates...");

    while let Some(delivery_result) = consumer.next().await {
        if let Ok(delivery) = delivery_result {
            let data = delivery.data.clone();
            if let Ok(update) = serde_json::from_slice::<TrafficUpdate>(&data) {
                println!("Received update: {:?}", update);
                if update.vehicle_count >= 4 {
                    let rec = Recommendation {
                        lane_id: update.lane_id,
                        new_green_time: 40,
                        timestamp: current_time_secs(),
                    };
                    publish_message(&channel, "recommendations", "", &rec).await;
                    let log = LogEvent {
                        source: "FlowAnalyzer".into(),
                        message: format!("Published recommendation for lane {}", update.lane_id),
                        timestamp: current_time_secs(),
                    };
                    publish_message(&channel, "logs", "", &log).await;
                }
            }
            delivery.ack(BasicAckOptions::default()).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_flow_analyzer().await {
        eprintln!("Error in flow analyzer: {}", e);
    }
}
