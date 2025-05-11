use lapin::{publisher_confirm::PublisherConfirm, Connection, ConnectionProperties};
use rocket::{Build, Rocket};
use std::env;

pub type RabbitChannel = lapin::Channel;

#[derive(serde::Serialize)]
struct Message<'r> {
    pub task_type: String,
    pub payload: Vec<&'r str>,
    pub task_id: String,
}

#[derive(serde::Serialize)]
struct MessageReview {
    pub task_type: String,
    pub payload: Vec<Vec<String>>,
    pub task_id: String,
}

pub async fn init_rabbit_queues(rocket: Rocket<Build>) -> Rocket<Build> {
    let connection_string =
        env::var("CONNECTION_STRING_RabbitMQ").expect("Failed to load RabbitMQ config");
    let conn = Connection::connect(&connection_string, ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");

    let channel = conn
        .create_channel()
        .await
        .expect("Failed to create channel RAbbitMQ");

    channel
        .queue_declare(
            "analysis_queue",
            lapin::options::QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            lapin::types::FieldTable::default(),
        )
        .await
        .expect("Failed to declare analysis_queue");

    rocket.manage(channel)
}

async fn send_message(
    channel: &lapin::Channel,
    payload: &Vec<u8>,
) -> Result<PublisherConfirm, lapin::Error> {
    channel
        .basic_publish(
            "",
            "analysis_queue",
            lapin::options::BasicPublishOptions::default(),
            payload,
            lapin::BasicProperties::default(),
        )
        .await
}

pub async fn send_task_to_text_analysis_queue(
    channel: &lapin::Channel,
    task_id: &uuid::Uuid,
    payload: Vec<&str>,
) -> Result<(), lapin::Error> {
    let task = Message {
        task_type: "text".to_string(),
        payload,
        task_id: task_id.to_string(),
    };
    let payload = serde_json::to_vec(&task).expect("Failed to serialize task to JSON");
    let _confirm = send_message(channel, &payload).await?;
    Ok(())
}

pub async fn send_task_to_photo_analysis_queue(
    channel: &lapin::Channel,
    task_id: &uuid::Uuid,
    payload: Vec<&str>,
) -> Result<(), lapin::Error> {
    let task = Message {
        task_type: "photo".to_string(),
        payload,
        task_id: task_id.to_string(),
    };
    let payload = serde_json::to_vec(&task).expect("Failed to serialize task to JSON");
    let _confirm = send_message(channel, &payload).await?;
    Ok(())
}

pub async fn send_task_to_reviews_analysis_queue(
    channel: &lapin::Channel,
    task_id: &uuid::Uuid,
    payload: Vec<Vec<String>>,
) -> Result<(), lapin::Error> {
    let task = MessageReview {
        task_type: "reviews".to_string(),
        payload,
        task_id: task_id.to_string(),
    };
    let payload = serde_json::to_vec(&task).expect("Failed to serialize task to JSON");
    let _confirm = send_message(channel, &payload).await?;
    Ok(())
}
