use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rdkafka::message::OwnedHeaders;
use governor::{Quota, RateLimiter, state::StreamRateLimitExt};
use rdkafka::producer::{FutureProducer, FutureRecord};
use futures::StreamExt;
use rdkafka::ClientConfig;

#[tokio::main]
async fn main() {
    let topic = "test";
    let bootstrap_servers = "10.32.10.100:9092";
    let limiter = Arc::new(RateLimiter::direct(Quota::per_second(NonZeroU32::new(3000).unwrap())));

    let mut handles = vec![];
    let threads = 32;

    for _i in 0..threads {
        let limiter = Arc::clone(&limiter);

        handles.push(tokio::spawn(async move {
            let mut client = ClientConfig::new();
            client.set("bootstrap.servers", bootstrap_servers);
            client.set("produce.offset.report", "true");
            client.set("message.timeout.ms", "5000");
            client.set("auto.commit.interval.ms", "1");
            let producer: FutureProducer = client.create().expect("Producer creation error");

            let payload = r#"ANALYSIS,2022-03-22 18:21:05.364,2,{"condition":{"push_lng":116.415556,"result_msg":"","real_lng":116.42178320894674,"push_lat":39.868558,"result_code":200,"source_type":"REALTIME","bike_type":0,"area_id":27,"bike_no":"9510406882","location_time":1647944348000,"real_lat":39.86995155417725},"entityMata":{"ipAddress":"10.111.152.4","logValue":0.0,"metric":"gov.lion.push.noncycling.position","service":"AppHellobikeGovService"}}"#.to_string();
            let mut stream = futures::stream::repeat(()).ratelimit_stream(&limiter);

            while stream.next().await.is_some() {
                let ts = Utc::now().timestamp_millis();
                let record = FutureRecord::to(&topic)
                    .payload(&payload)
                    .key("key")
                    .timestamp(ts)
                    .headers(OwnedHeaders::new().add("foo", "bar"));

                match producer.send(record, Duration::from_secs(0)).await {
                    Ok((_partition, _offset)) => {
                        // dbg!("partition: {}, offset: {}", partition, offset);
                    }
                    Err((err, _msg)) => {
                        panic!("Cannot send message to Kafka, err: {:?}", err)
                    }
                }
            }
        }))
    }

    futures::future::join_all(handles).await;
}
