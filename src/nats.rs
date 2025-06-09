use async_nats::{
    jetstream::{
        self,
        consumer::{
            pull::{Config as pullConfig, Stream as pullStream},
            PullConsumer,
        },
        stream::{Config, Stream},
    },
    Error,
};
use rocket::{Build, Rocket, State};
use std::env;

pub type NatsStream = Stream;

#[derive(serde::Serialize)]
pub struct Message {
    pub task_type: String,
    pub payload: String,
    pub task_id: String,
}

pub async fn init_connection_to_stream(rocket: Rocket<Build>) -> Rocket<Build> {
    let connection_string = env::var("CONNECTION_STRING_NATS").expect("Failed to load NATS config");
    let stream_name = crate::STREAM_NAME;
    let client = async_nats::connect(connection_string)
        .await
        .expect("Can`t connect to NATS server");

    let jetstream = jetstream::new(client);
    let stream = jetstream
        .get_or_create_stream(Config {
            name: stream_name.to_string().to_uppercase(),
            subjects: vec![format!("{}.>", stream_name).into()],
            retention: jetstream::stream::RetentionPolicy::WorkQueue,
            ..Default::default()
        })
        .await
        .expect("Failed to create or get stream");
    rocket.manage(stream)
}

pub async fn get_messages_stream(
    stream: &State<NatsStream>,
    id: uuid::Uuid,
) -> Result<pullStream, Error> {
    let stream_name = crate::STREAM_NAME;
    let consumer_name = format!("pull-{}", id);
    let filter_subject = format!("{}.{}", stream_name, id);
    let consumer: PullConsumer = stream
        .get_or_create_consumer(
            &consumer_name,
            pullConfig {
                durable_name: Some(consumer_name.clone()),
                filter_subject: filter_subject.into(),
                ..Default::default()
            },
        )
        .await?;
    Ok(consumer.messages().await?)
}
