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

#[tokio::main]
async fn main() {
    let channel = create_channel().await;
    declare_exchange(&channel, "logs", lapin::ExchangeKind::Fanout).await;

    // Create a temporary queue and bind it to the logs exchange.
    let queue = channel.queue_declare("", QueueDeclareOptions::default(), FieldTable::default())
        .await.expect("Queue declare failed");
    channel.queue_bind(queue.name().as_str(), "logs", "", QueueBindOptions::default(), FieldTable::default())
        .await.expect("Queue bind failed");

    let mut consumer = channel.basic_consume(queue.name().as_str(), "system_monitoring", BasicConsumeOptions::default(), FieldTable::default())
        .await.expect("Failed to create consumer");

    println!("System Monitoring waiting for log messages...");

    while let Some(delivery) = consumer.next().await {
        if let Ok((channel, delivery)) = delivery {
            let data = delivery.data;
            if let Ok(log) = serde_json::from_slice::<LogEvent>(&data) {
                println!("[Time: {}] {}: {}", log.timestamp, log.source, log.message);
            }
            channel.basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .await.expect("Ack failed");
        }
    }
}
