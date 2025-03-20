use chrono::{DateTime, Utc};
use serde::Deserialize;

use kubernetes::{ObjectMeta, Resource};

/// Event is a report of an event somewhere in the cluster. Events have a limited retention
/// time and triggers and messages may evolve with time. Event consumers should not rely on
/// the timing of an event with a given Reason reflecting reflecting a consistent underlying
/// trigger, or the continued existence of events with that Reason. Events should be treated
/// as informative, best-effort, supplemental data.
///
/// See: https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#event-v1-core
#[derive(Debug, Deserialize)]
pub struct Event {
    /// Standard object's metadata.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: ObjectMeta,

    /// What action was taken/failed regarding to the Regarding object.
    pub action: Option<String>,

    /// This should be a short, machine understandable string that gives the reason for the
    /// transition into the object's current status.
    pub reason: Option<String>,

    /// A human-readable description of the status of this operation.
    pub message: Option<String>,

    /// Type of this event (Normal, Warning), new types could be added in the future
    #[serde(rename = "type")]
    pub typ: Option<String>,

    /// Time when this Event was first observed.
    ///
    /// The value is a version of timestamp with microsecond level precision.
    /// e.g. `1998-05-05T05:05:05.000000Z`
    #[serde(rename = "eventTime")]
    pub event_time: Option<DateTime<Utc>>,

    /// `firstTimestamp` is the deprecated field assuring backward compatibility
    /// with core.v1 Event type.
    #[serde(rename = "firstTimestamp")]
    pub first_timestamp: Option<DateTime<Utc>>,

    /// `lastTimestamp` is the deprecated field assuring backward compatibility
    /// with core.v1 Event type.
    #[serde(rename = "lastTimestamp")]
    pub last_timestamp: Option<DateTime<Utc>>,
}

impl Resource for Event {
    const GROUP: &'static str = "events.k8s.io";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "Event";

    const PLURAL: &'static str = "events";
}

impl Event {
    /// Return the EventTimestamp based on the populated k8s event timestamps.
    ///
    /// Priority: EventTime > LastTimestamp > FirstTimestamp
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        if self.event_time.is_some() {
            return self.event_time;
        }
        if self.last_timestamp.is_some() {
            return self.last_timestamp;
        }
        if self.first_timestamp.is_some() {
            return self.first_timestamp;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let text = r#"
        {
            "apiVersion": "v1",
            "count": 1,
            "eventTime": null,
            "firstTimestamp": "2024-07-21T10:36:25Z",
            "involvedObject": {
                "apiVersion": "apps/v1",
                "kind": "Deployment",
                "name": "route",
                "namespace": "default",
                "resourceVersion": "744",
                "uid": "7b29c6d4-145c-4fff-a4b7-8e7df8c706f1"
            },
            "kind": "Event",
            "lastTimestamp": "2024-07-21T10:36:25Z",
            "message": "Scaled up replica set route-945474c85 to 1",
            "metadata": {
                "creationTimestamp": "2024-07-21T10:36:25Z",
                "name": "route.17e4340c71ae7b20",
                "namespace": "default",
                "resourceVersion": "747",
                "uid": "17c3691b-ae54-412b-bb6f-2073a41ff661"
            },
            "reason": "ScalingReplicaSet",
            "reportingComponent": "deployment-controller",
            "reportingInstance": "",
            "source": {
                "component": "deployment-controller"
            },
            "type": "Normal"
        }
        "#;

        let event = serde_json::from_str::<Event>(text).unwrap();

        assert_eq!(event.action, None);
        assert_eq!(event.event_time, None);
        assert_eq!(event.reason, Some("ScalingReplicaSet".into()));
        assert_eq!(
            event.message,
            Some("Scaled up replica set route-945474c85 to 1".into())
        );
        assert_eq!(event.typ, Some("Normal".into()));
    }

    #[test]
    fn url() {
        assert_eq!(Event::url_path(None), "/apis/events.k8s.io/v1/events");
        assert_eq!(
            Event::url_path(Some("foo")),
            "/apis/events.k8s.io/v1/namespaces/foo/events"
        );
    }
}
