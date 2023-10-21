#![allow(warnings, clippy::all, clippy::pedantic)]

mod log;
mod metadata;
mod metric;

mod proto_event {
    include!(concat!(env!("OUT_DIR"), "/event.rs"));
}

use std::borrow::Cow;
use std::collections::BTreeMap;

use chrono::TimeZone;
pub use proto_event::*;
use tracing::error;

use super::{Key, LogRecord, MetricValue, Tags};
use crate::metadata::WithMetadata;
use crate::proto::event_wrapper::Event;
use crate::tags::Array;

impl From<Log> for Event {
    fn from(log: Log) -> Self {
        Self::Log(log)
    }
}

impl From<Metric> for Event {
    fn from(metric: Metric) -> Self {
        Self::Metric(metric)
    }
}

impl From<Event> for EventWrapper {
    fn from(event: Event) -> Self {
        Self { event: Some(event) }
    }
}

impl From<EventWrapper> for crate::Event {
    fn from(wrapper: EventWrapper) -> Self {
        let event = wrapper.event.unwrap();

        match event {
            Event::Log(log) => Self::Log(log.into()),
            Event::Metric(metric) => Self::Metric(metric.into()),
        }
    }
}

impl events::Events {
    fn from_logs(logs: crate::Logs) -> Self {
        let logs = logs.into_iter().map(Into::into).collect();
        Self::Logs(events::Logs { logs })
    }

    fn from_metrics(metrics: crate::Metrics) -> Self {
        let metrics = metrics.into_iter().map(Into::into).collect();
        Self::Metrics(events::Metrics { metrics })
    }
}

impl From<crate::Events> for Events {
    fn from(events: crate::Events) -> Self {
        let events = Some(match events {
            crate::Events::Logs(logs) => events::Events::from_logs(logs),
            crate::Events::Metrics(metrics) => events::Events::from_metrics(metrics),
            crate::Events::Traces(_) => unimplemented!(),
        });

        Self { events }
    }
}

impl From<Events> for crate::Events {
    fn from(events: Events) -> Self {
        let events = events.events.unwrap();

        match events {
            events::Events::Logs(logs) => {
                crate::Events::Logs(logs.logs.into_iter().map(Into::into).collect())
            }
            events::Events::Metrics(metrics) => {
                crate::Events::Metrics(metrics.metrics.into_iter().map(Into::into).collect())
            }
        }
    }
}
