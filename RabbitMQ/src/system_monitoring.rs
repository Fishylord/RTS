// system_monitoring.rs
use tokio;
use lapin::{options::*, types::FieldTable};
use futures_util::stream::StreamExt;
use serde::{Serialize, Deserialize};

mod mq;
use mq::{create_channel, declare_exchange};

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEvent {
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

pub async fn run_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    let channel = create_channel().await;
    declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;

    let queue = channel.queue_declare("", QueueDeclareOptions::default(), FieldTable::default())
        .await?;
    channel.queue_bind(queue.name().as_str(), "logs", "", QueueBindOptions::default(), FieldTable::default())
        .await?;

    let mut consumer = channel.basic_consume(queue.name().as_str(), "system_monitoring", BasicConsumeOptions::default(), FieldTable::default())
        .await?;

    println!("System Monitoring waiting for log messages...");

    while let Some(delivery_result) = consumer.next().await {
        if let Ok(delivery) = delivery_result {
            let data = delivery.data.clone();
            if let Ok(log) = serde_json::from_slice::<LogEvent>(&data) {
                println!("[Time: {}] {}: {}", log.timestamp, log.source, log.message);
            }
            delivery.ack(BasicAckOptions::default()).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_monitoring().await {
        eprintln!("Error in system monitoring: {}", e);
    }
}
