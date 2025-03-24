use chrono::{DateTime, Utc};
use serde::Deserialize;

use kubernetes::{ObjectMeta, Resource};

/// ObjectReference contains enough information to let your inspect or modify the referred object.
#[derive(Debug, Deserialize)]
pub struct ObjectReference {
    /// API version of the referent
    #[serde(default, rename = "apiVersion")]
    pub api_version: Option<String>,

    /// If referring to a piece of an object instead of an entire object, this string should
    /// contain a valid JSON/Go field access statement, such as
    /// desiredState.manifest.containers[2]. For example, if the object reference is to a
    /// container within a pod, this would take on a value like: "spec.containers{name}"
    /// (where "name" refers to the name of the container that triggered the event) or if no
    /// container name is specified "spec.containers[2]" (container with index 2 in this pod).
    /// This syntax is chosen only to have some well-defined way of referencing a part of
    /// an object.
    #[serde(default, rename = "field_path")]
    pub field_path: Option<String>,

    /// Kind of the referent.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds
    #[serde(default)]
    pub kind: Option<String>,

    /// Name of the referent.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names
    #[serde(default)]
    pub name: Option<String>,

    /// Namespace of the referent.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/namespaces/
    #[serde(default)]
    pub namespace: Option<String>,

    /// Specific resourceVersion to which this reference is made, if any.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#concurrency-control-and-consistency
    #[serde(default)]
    pub resource_version: Option<String>,

    /// UID of the referent.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#uids
    #[serde(default)]
    pub uid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventSeries {
    /// count is the number of occurrences in this series up to the last heartbeat time.
    pub count: i64,

    /// lastObservedTime is the time when last Event from the series was seen before last heartbeat.
    #[serde(default, rename = "lastObservedTime")]
    pub last_observed_time: Option<DateTime<Utc>>,
}

/// Event is a report of an event somewhere in the cluster. Events have a limited retention
/// time and triggers and messages may evolve with time. Event consumers should not rely on
/// the timing of an event with a given Reason reflecting reflecting a consistent underlying
/// trigger, or the continued existence of events with that Reason. Events should be treated
/// as informative, best-effort, supplemental data.
///
/// See: https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#event-v1-core
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Event {
    /// Standard object's metadata.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: ObjectMeta,

    /// What action was taken/failed regarding to the Regarding object.
    pub action: Option<String>,

    /// APIVersion defines the versioned schema of this representation of an object. Servers
    /// should convert recognized schemas to the latest internal value, and may reject
    /// unrecognized values.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#resources
    #[serde(default, rename = "apiVersion")]
    pub api_version: Option<String>,

    /// Time when this Event was first observed.
    ///
    /// The value is a version of timestamp with microsecond level precision.
    /// e.g. `1998-05-05T05:05:05.000000Z`
    #[serde(rename = "eventTime")]
    pub event_time: Option<DateTime<Utc>>,

    /// Kind is a string value representing the REST resource this object represents. Servers
    /// may infer this from the endpoint the client submits requests to. Cannot be updated. In
    /// CamelCase.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds
    #[serde(default)]
    pub kind: Option<String>,

    /// note is a human-readable description of the status of this operation. Maximal length of
    /// the note is 1kB, but libraries should be prepared to handle values up to 64kB.
    #[serde(default)]
    pub note: Option<String>,

    /// This should be a short, machine understandable string that gives the reason for the
    /// transition into the object's current status.
    #[serde(default)]
    pub reason: Option<String>,

    /// regarding contains the object this Event is about. In most cases it's an Object reporting
    /// controller implements, e.g. ReplicaSetController implements ReplicaSets and this event
    /// is emitted because it acts on some changes in a ReplicaSet object.
    #[serde(default)]
    pub regarding: Option<ObjectReference>,

    /// Optional secondary object for more complex actions.
    #[serde(default)]
    pub related: Option<ObjectReference>,

    /// reportingController is the name of the controller that emitted this Event,
    /// e.g. `kubernetes.io/kubelet`. This field cannot be empty for new Events.
    #[serde(default, rename = "reportingController")]
    pub reporting_controller: Option<String>,

    /// ID of the controller instance, e.g. `kubelet-xyzf`.
    #[serde(rename = "reportingInstance")]
    pub reporting_instance: Option<String>,

    /// series is data about the Event series this event represents or nil if it's a singleton Event.
    #[serde(default)]
    pub series: Option<EventSeries>,

    /// Type of this event (Normal, Warning), new types could be added in the future
    #[serde(rename = "type")]
    pub typ: Option<String>,

    /// `firstTimestamp` is the deprecated field assuring backward compatibility
    /// with core.v1 Event type.
    #[serde(rename = "deprecatedFirstTimestamp")]
    pub deprecated_first_timestamp: Option<DateTime<Utc>>,

    /// `lastTimestamp` is the deprecated field assuring backward compatibility
    /// with core.v1 Event type.
    #[serde(rename = "deprecatedLastTimestamp")]
    pub deprecated_last_timestamp: Option<DateTime<Utc>>,
}

impl Resource for Event {
    const GROUP: &'static str = "events.k8s.io";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "Event";

    const PLURAL: &'static str = "events";
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
            "note": "Scaled up replica set route-945474c85 to 1",
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
            event.note,
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
