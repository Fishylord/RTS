// mq.rs
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, Channel, ExchangeKind, BasicProperties};
use tokio_amqp::*;
use serde::Serialize;
use serde_json;

/// Create a RabbitMQ channel using a connection string from the AMQP_ADDR environment variable.
pub async fn create_channel() -> Channel {
    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());
    let connection = Connection::connect(&addr, ConnectionProperties::default().with_tokio())
        .await
        .expect("Failed to connect to RabbitMQ");
    connection.create_channel().await.expect("Failed to create channel")
}

/// Publish a serializable message to the specified exchange and routing key.
pub async fn publish_message<T: Serialize>(channel: &Channel, exchange: &str, routing_key: &str, message: &T) {
    let payload = serde_json::to_vec(message).expect("Failed to serialize message");
    channel
        .basic_publish(
            exchange,
            routing_key,
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await
        .expect("Failed to publish message")
        .await
        .expect("Publish not confirmed");
}

/// Declare an exchange if it does not already exist.
pub async fn declare_exchange(channel: &Channel, exchange: &str, kind: ExchangeKind) {
    channel
        .exchange_declare(
            exchange,
            kind,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Failed to declare exchange");
}
