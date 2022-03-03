use std::collections::BTreeMap;

/// TargetGroup is a set of targets with a common tags
pub struct TargetGroup {
    /// `targets` is a list of targets identified by a label set. Each target
    /// is uniquely identifiable in the group by its `address` label
    pub targets: Vec<BTreeMap<String, String>>,

    /// `labels` is a set of labels that is common across all targets in the group
    pub labels: BTreeMap<String, String>,

    /// An identifier that describes a group of targets
    pub source: Option<String>,
}

/// Discoverer provides information about target groups. It maintains a set of
/// sources from which TargetGroup can originate.
///
/// `Discoverer` does not know if an actual change happened. It does guarantee
/// that it sends the new TargetGroup whenever a change happens.
pub trait Discoverer {
    fn targets(&self) -> Vec<TargetGroup>;
}
