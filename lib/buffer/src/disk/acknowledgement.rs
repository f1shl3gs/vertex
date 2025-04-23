use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::queue::Queue;

#[derive(Debug)]
enum PendingMarkerLength {
    Known(u64),
    Assumed(u64),
    Unknown,
}

struct PendingMarker<T> {
    id: u64,
    len: PendingMarkerLength,
    data: Option<T>,
}

impl<T: Debug> Debug for PendingMarker<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingMarker")
            .field("id", &self.id)
            .field("len", &self.len)
            .field("data", &self.data)
            .finish()
    }
}

/// The length of an eligible marker
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub enum EligibleMarkerLength {
    /// The marker's length was declared upfront when added.
    Known(u64),

    /// The marker's length was calculated based on imperfect information, and so while
    /// it should accurately represent a correct range that covers any gaps in the marker
    /// range, it may or may not represent one true marker, or possibly multiple markers.
    Assumed(u64),
}

/// A marker that has been fully acknowledged
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct EligibleMarker<T> {
    #[allow(dead_code)]
    pub id: u64,
    pub len: EligibleMarkerLength,
    pub data: Option<T>,
}

/// Error returned by `OrderedAcknowledgement::add_marker`
///
/// In general, this error represents a breaking ID monotonicity, or more likely, the loss of
/// records where the entire records may have been skipped as an attempted add provides an ID
/// that not the next expected ID.
///
/// While the exact condition must be determined by the caller, we attempt to provide as much
/// information as we reasonably based on the data we have, whether it's simply that the ID
/// didn't match the next expected ID, or that we know it is definitively ahead or behind the
/// next expected ID.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub enum MarkerError {
    /// The given marker ID is behind the next expected marker ID.
    ///
    /// As `OrderedAcknowledgements` expects monotonic marker IDs, this represents a violation
    /// of the acknowledgement state, and must be handled by the caller. Generally speaking,
    /// this is an unrecoverable error.
    MonotonicityViolation,
}

/// Result of comparing a potential pending marker ID with the expected next pending
/// marker ID.
pub enum MarkerOffset {
    /// The given marker ID is aligned with the next expected marker ID.
    Aligned,

    /// The given marker ID is ahead of the next expected marker ID.
    ///
    /// When the last pending marker has a fixed-size, we can calculate the exact
    /// marker ID that we expect to see next. In turn, we can tell how far ahead
    /// the given marker ID from the next expected marker ID.
    ///
    /// The next expected marker ID, and the amount (gap) that the given marker ID
    /// and the next expected marker ID differ, are provided.
    Gap(u64, u64),

    /// The given marker ID may or may not be aligned.
    ///
    /// This occurs when the last pending marker has an unknown size, as we cannot
    /// determine whether the given marker ID is the next true marker ID without
    /// knowing where the last pending marker should end.
    ///
    /// The last pending marker ID is provided.
    NotEnoughInformation(u64),

    /// The given marker ID is behind the next expected marker ID.
    ///
    /// As `OrderedAcknowledgements` expects monotonic marker IDs, this represents
    /// a violation of the acknowledgement state, and must be handled by the caller.
    /// Generally speaking, this is an unrecoverable error.
    MonotonicityViolation,
}

/// `OrderedAcknowledgements` allows determining when a record is eligible for deletion.
///
/// ### Purpose
///
/// In disk buffer, a record may potentially represent multiple events. As these events
/// may be processed at different times by a sink, and in a potentially different order
/// than when stored in the record, a record cannot be considered fully processed until
/// all the events have been accounted for. As well, only once a record has been fully
/// processed can it be considered for deletion to free up space in the buffer.
///
/// To complicate matters, a record may sometimes not be decodable -- on-disk corruption,
/// invalid encoding scheme that is no longer supported, etc. -- but still needs to be
/// accounted for to know when it can be deleted, and so that the correct metrics can be
/// generated to determine how many events were lost by the record not being able to be
/// processed normally.
///
/// ### Functionality
///
/// `OrderedAcknowledgements` provides the ability to add "markers", which are a virtual
/// token mapped to a record. Markers track the ID of a record, how long the record is
/// (if known), and optional data that is specific to the record. It also provides the
/// ability to add acknowledgements which can then be consumed to allow yielding makers
/// which have collected enough acknowledgements and are thus "eligible".
///
/// ### Detecting record gaps and the length of undecodable records
///
/// Additionally, and as hinted at above, markers can be added without a known length:
/// this may happen when a record is read, but it cannot be decoded, and thus determining
/// the true length is not possible.
///
/// When markers that have an unknown length are added, `OrderedAcknowledgements` will do
/// one of two things:
/// - figure out if the marker is ahead of the next expected marker ID, and add a synthetic
///   "gap" marker to compensate
/// - update the unknown length with an assumed length, based on the difference between its
///   ID and the next marker that gets added
///
/// In this way, `OrderedAcknowledgements` provides a contiguous range of marker IDs, which
/// allows detecting not only the presumed length of a record that couldn't be decoded, but
/// also if any records were deleted from disk or unable to be read at all. Based on the
/// invariant of expecting IDs to be monotonic and contiguous, we know that if we expect our
/// next marker ID to be 5, but instead get one with an ID of 8, that there's 3 missing events
/// in the middle that have not been accounted for.
///
/// Similarly, even when we don't know what the next expected marker ID should be, we can
/// determine the number of events that were lost when the next marker is added, as marker
/// IDs represent the start of a record, and so simple arithmetic can determine the number
/// of events that have theoretically been lost.
pub struct OrderedAcknowledgements<T> {
    unclaimed: AtomicU64,
    watermark: AtomicU64,
    pending_markers: Arc<Queue<PendingMarker<T>>>,
}

impl<T: Debug> Debug for OrderedAcknowledgements<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderedAcknowledgements")
            .field("unclaimed", &self.unclaimed)
            .field("watermark", &self.watermark)
            .finish()
    }
}

impl<T: Debug> OrderedAcknowledgements<T> {
    pub fn from_acked(watermark: u64) -> Self {
        Self {
            unclaimed: AtomicU64::default(),
            watermark: AtomicU64::new(watermark),
            pending_markers: Arc::new(Queue::default()),
        }
    }

    /// Adds the given number of acknowledgements.
    ///
    /// Acknowledgements should be given by the caller to update the acknowledgement
    /// state before trying to get any eligible markers.
    #[cfg_attr(test, tracing::instrument(skip(self)))]
    #[inline]
    pub fn add_acknowledgements(&self, amount: u64) {
        self.unclaimed.fetch_add(amount, Ordering::Release);
    }

    /// Adds a marker
    ///
    /// The marker is tracked internally, and once the acknowledgement state has been advanced
    /// enough such that it is at or ahead of marker, the marker will become eligible.
    ///
    /// ## Gap detection and unknown length markers
    ///
    /// When a gap is detected between the given marker ID and the next expected marker ID, we
    /// insert a synthetic marker to represent that gap. For example, if we had marker with an
    /// ID of 0 and length of 5, we would expect the next marker to have an ID of 5. If instead,
    /// a marker with an ID of 7 was given, that would represent a gap of 2. We insert a synthetic
    /// maker with an ID of 5 and a length of 2 before adding the marker with the ID of 7. This
    /// keeps the marker range contiguous and allows getting an eligible marker for the gap so
    /// the caller can detect that a gap occurred.
    ///
    /// Likewise, when a caller inserts an unknown length marker, we cannot know its length
    /// until the next marker is added. When that happens, we assume the given marker ID is
    /// monotonic, and thus that the length of the previous marker, which has an unknown
    /// length, must have a length equal to the difference between the given marker ID and
    /// the unknown length marker ID. We update the unknown length marker to reflect this.
    ///
    /// In both cases, the markers will have a length that indicates that the amount represents
    /// a gap, and not a marker that was directly added by the caller themselves.
    ///
    /// ## Errors
    ///
    /// When other pending markers are present, and the given ID is logically behind the next
    /// expected marker ID, `Err(Error::MonotonicityViolation)` is returned.
    ///
    /// # Panics
    ///
    /// Panics if pending markers is empty when last pending marker is an unknown size.
    #[cfg_attr(test, tracing::instrument(skip(self, data)))]
    pub fn add_marker(
        &self,
        id: u64,
        len: Option<u64>,
        data: Option<T>,
    ) -> Result<(), MarkerError> {
        // First, figure out where this given marker ID stands compared to our next expected
        // marker ID, and the pending marker state in general.
        match self.get_marker_id_offset(id) {
            // We have enough information to determine the given marker ID is the next
            // expected marker ID, so we can proceed normally.
            MarkerOffset::Aligned => {}
            // The last pending marker is fixed-size, and the given marker ID is past where
            // that marker ends, so we need to inject a synthetic gap marker to compensate
            // for that
            MarkerOffset::Gap(expected, amount) => {
                self.pending_markers.push(PendingMarker {
                    id: expected,
                    len: PendingMarkerLength::Assumed(amount),
                    data: None,
                });
            }

            // The last pending marker is an unknown size, so we're using this given marker
            // ID to calculate the length of that last pending marker, and in turn, we're
            // going to adjust its length before adding the new pending marker.
            MarkerOffset::NotEnoughInformation(last_marker_id) => {
                let len = id.wrapping_sub(last_marker_id);
                let last_marker = self
                    .pending_markers
                    .tail_mut()
                    .unwrap_or_else(|| panic!("pending markers should have items"));

                last_marker.len = PendingMarkerLength::Assumed(len);
            }

            // We detected a monotonicity violation, which we can't do anything about, so just
            // immediately inform the caller
            MarkerOffset::MonotonicityViolation => return Err(MarkerError::MonotonicityViolation),
        }

        self.pending_markers.push(PendingMarker {
            id,
            len: len.map_or(PendingMarkerLength::Unknown, PendingMarkerLength::Known),
            data,
        });

        Ok(())
    }

    /// Gets the next marker which has been fully acknowledged.
    ///
    /// A pending marker becomes eligible when the acknowledged marker ID is at or past
    /// the pending marker ID plus the marker length.
    ///
    /// For pending markers with an unknown length, another pending marker must be present
    /// after it in order to calculate the ID offsets and determine the marker length.
    pub fn get_next_eligible_marker(&self) -> Option<EligibleMarker<T>> {
        let unclaimed = self.unclaimed.load(Ordering::Acquire);
        let effective_acked_marker_id = self
            .watermark
            .load(Ordering::Acquire)
            .wrapping_add(unclaimed);

        let maybe_eligible_marker =
            self.pending_markers
                .head()
                .and_then(|marker| match marker.len {
                    // If the acked marker ID is ahead of this marker, plus its length, it's
                    // been fully acknowledged, and we can consume and yield the marker. We
                    // have to double verify this by checking that there's enough unclaimed
                    // acknowledgements to support this length because otherwise we might
                    // fall victim to markers that simply generate a required acked marker
                    // ID that is not enough for this marker but is enough to align
                    // the effective/required ID
                    PendingMarkerLength::Known(len) => {
                        let required_acked_marker_id = marker.id.wrapping_add(len);

                        if required_acked_marker_id <= effective_acked_marker_id && unclaimed >= len
                        {
                            Some((EligibleMarkerLength::Known(len), len))
                        } else {
                            None
                        }
                    }

                    // The marker has an assumed length, which means a marker was added after it,
                    // which implies that it is de facto eligible as unknown length markers do not
                    // consume acknowledgements and so are immediately eligible once an assumed
                    // length can be determined.
                    PendingMarkerLength::Assumed(len) => {
                        Some((EligibleMarkerLength::Assumed(len), u64::MIN))
                    }
                    // We don't yet know what the length is for this marker, so we're stuck waiting
                    // for another marker to be added before that can be determined.
                    PendingMarkerLength::Unknown => None,
                });

        // If we actually got an eligible marker, we need to actually remove it from the pending
        // marker queue and potentially adjust the amount of unclaimed acknowledgements we have
        match maybe_eligible_marker {
            Some((len, acknowledged)) => {
                // If we actually got an eligible marker, we need to actually remove it from
                // the pending marker queue, potentially adjust the amount of unclaimed
                // acknowledgements we have, and adjust our acked marker ID.
                let Some(PendingMarker { id, data, .. }) = self.pending_markers.pop().unwrap()
                else {
                    unreachable!("pending markers should not be empty")
                };

                if acknowledged > 0 {
                    self.unclaimed.fetch_sub(acknowledged, Ordering::Acquire);
                }

                self.watermark.store(
                    id.wrapping_add(match len {
                        EligibleMarkerLength::Known(len) => len,
                        EligibleMarkerLength::Assumed(len) => len,
                    }),
                    Ordering::Release,
                );

                Some(EligibleMarker { id, len, data })
            }
            None => None,
        }
    }

    /// Gets the marker ID offset for the given ID.
    ///
    /// If the given ID matches our next expected marker ID, then `MarkerOffset::Aligned` is
    /// returned.
    ///
    /// Otherwise, we return one of the following variants:
    /// - if we have no pending markers, `MarkerOffset::Gap` is returned, and contains the
    ///   delta between the given ID and the next expected marker ID.
    /// - if we have pending markers, and the given ID is logically behind the next expected
    ///   marker ID, `MarkerOffset::MonotonicityViolation` is returned, indicating that the
    ///   monotonicity invariant has been violated
    /// - if we have pending markers, and the given ID is logically ahead of the next expected
    ///   marker, `MarkerOffset::Gap` is returned, specifying how far ahead of the next
    ///   expected marker ID it is
    /// - if we have pending markers, and the last pending marker has an unknown length,
    ///   `MarkerOffset::NotEnoughInformation` is returned, as we require a fixed-size marker
    ///   to correctly calculate the next expected marker ID
    fn get_marker_id_offset(&self, id: u64) -> MarkerOffset {
        if self.pending_markers.is_empty() {
            // We have no pending markers, but our acknowledged ID offset should match the marker
            // ID being given here, otherwise it would imply that the markers were not contiguous.
            //
            // We return the difference between the ID and our acknowledged ID offset with the
            // assumption that the new ID is monotonic. Since IDs wraparound, we don't bother
            // looking at if it's higher or lower because we can't reasonably tell if this record
            // ID is actually correct but other markers in between went missing, etc.
            //
            // Basically, it's up to the caller to figure this out. We're just trying to give
            // them as much information as we can.
            let watermark = self.watermark.load(Ordering::Acquire);
            if watermark != id {
                return MarkerOffset::Gap(watermark, id.wrapping_sub(watermark));
            }
        } else {
            let back = self
                .pending_markers
                .tail()
                .expect("pending markers should have items");

            // When we know the length of the previously added pending marker, we can figure out
            // where this marker's ID should land, as we do not allow for noncontiguous marker
            // ID range.
            if let PendingMarkerLength::Known(len) = back.len {
                // If we know the length of the back item, then we know exactly what the ID for
                // the next marker to follow it should be. If this incoming marker doesn't match,
                // something is wrong.
                let expected_next = back.id.wrapping_add(len);
                if id != expected_next {
                    if expected_next < back.id && id < expected_next {
                        return MarkerOffset::MonotonicityViolation;
                    }

                    return MarkerOffset::Gap(expected_next, id.wrapping_sub(expected_next));
                }
            } else {
                // without a fixed-size marker, we cannot be sure whether this marker ID is
                // aligned or not
                return MarkerOffset::NotEnoughInformation(back.id);
            }
        }

        MarkerOffset::Aligned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo() {
        let acker: OrderedAcknowledgements<()> = OrderedAcknowledgements::from_acked(1);
        println!("init\n{:#?}", acker);

        acker.add_marker(1, Some(25), None).unwrap();
        println!("{:#?}", acker);

        while let Some(mark) = acker.get_next_eligible_marker() {
            println!("{:?}", mark);
        }

        acker.add_marker(26, Some(1), None).unwrap();
        println!("{:#?}", acker);

        acker.add_acknowledgements(25);

        while let Some(mark) = acker.get_next_eligible_marker() {
            println!("{:?}", mark);
        }
        // println!("{:#?}", acker);
    }

    #[derive(Debug, Clone, Copy)]
    enum Action {
        Acknowledge(u64),
        AddMarker((u64, Option<u64>)),
        GetNextEligibleMarker,
    }

    #[derive(Debug, PartialEq)]
    enum ActionResult {
        // Number of unclaimed acknowledgements.
        Acknowledge(u64),
        AddMarker(Result<(), MarkerError>),
        GetNextEligibleMarker(Option<EligibleMarker<()>>),
    }

    fn apply_action_sut(sut: &mut OrderedAcknowledgements<()>, action: Action) -> ActionResult {
        match action {
            Action::Acknowledge(amount) => {
                sut.add_acknowledgements(amount);
                ActionResult::Acknowledge(sut.unclaimed.load(Ordering::Acquire))
            }
            Action::AddMarker((id, maybe_len)) => {
                let result = sut.add_marker(id, maybe_len, None);
                ActionResult::AddMarker(result)
            }
            Action::GetNextEligibleMarker => {
                let result = sut.get_next_eligible_marker();
                ActionResult::GetNextEligibleMarker(result)
            }
        }
    }

    macro_rules! step {
        ($action_name:ident, result => $result_input:expr) => {
            (
                Action::$action_name,
                ActionResult::$action_name($result_input),
            )
        };
        ($action_name:ident, input => $action_input:expr, result => $result_input:expr) => {
            (
                Action::$action_name($action_input),
                ActionResult::$action_name($result_input),
            )
        };
    }

    #[test]
    fn basic_cases() {
        // Smoke test.
        run_test_case("empty", vec![step!(GetNextEligibleMarker, result => None)]);

        // Simple through-and-through:
        run_test_case(
            "through_and_through",
            vec![
                step!(AddMarker, input => (0, Some(5)), result => Ok(())),
                step!(Acknowledge, input => 5, result => 5),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Known(5), data: None,
                    }
                )),
            ],
        );
    }

    #[test]
    fn invariant_cases() {
        // Checking for an eligible record between incremental acknowledgement:
        run_test_case(
            "eligible_multi_ack",
            vec![
                step!(AddMarker, input => (0, Some(13)), result => Ok(())),
                step!(Acknowledge, input => 5, result => 5),
                step!(GetNextEligibleMarker, result => None),
                step!(Acknowledge, input => 5, result => 10),
                step!(GetNextEligibleMarker, result => None),
                step!(Acknowledge, input => 5, result => 15),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Known(13), data: None,
                    }
                )),
            ],
        );

        // Unknown length markers can't be returned until a marker exists after them, even if we
        // could maximally acknowledge them:
        run_test_case(
            "unknown_len_no_subsequent_marker",
            vec![
                step!(AddMarker, input => (0, None), result => Ok(())),
                step!(Acknowledge, input => 5, result => 5),
                step!(GetNextEligibleMarker, result => None),
            ],
        );

        // We can always get back an unknown marker, with its length, regardless of
        // acknowledgements, so long as there's a marker exists after them: fixed.
        run_test_case(
            "unknown_len_subsequent_marker_fixed",
            vec![
                step!(AddMarker, input => (0, None), result => Ok(())),
                step!(AddMarker, input => (5, Some(1)), result => Ok(())),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Assumed(5), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => None),
            ],
        );

        // We can always get back an unknown marker, with its length, regardless of
        // acknowledgements, so long as there's a marker exists after them: unknown.
        run_test_case(
            "unknown_len_subsequent_marker_unknown",
            vec![
                step!(AddMarker, input => (0, None), result => Ok(())),
                step!(AddMarker, input => (5, None), result => Ok(())),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Assumed(5), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => None),
            ],
        );

        // Can add a marker without a known length and it will generate a synthetic gap marker
        // that is immediately eligible:
        run_test_case(
            "unknown_len_no_pending_synthetic_gap",
            vec![
                step!(AddMarker, input => (1, None), result => Ok(())),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Assumed(1), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => None),
            ],
        );

        // When another marker exists, and is fixed size, we correctly detect when trying to add
        // another marker whose ID comes before the last pending marker we have:
        run_test_case(
            "detect_monotonicity_violation",
            vec![
                step!(AddMarker, input => (u64::MAX, Some(3)), result => Ok(())),
                step!(AddMarker, input => (1, Some(2)), result => Err(MarkerError::MonotonicityViolation)),
            ],
        );

        // When another marker exists, and is fixed size, we correctly detect when trying to add
        // another marker whose ID comes after the last pending marker we have, including the
        // length of the last pending marker, by updating the marker's unknown length to an
        // assumed length, which is immediately eligible:
        run_test_case(
            "unknown_len_updated_fixed_marker",
            vec![
                step!(AddMarker, input => (0, Some(4)), result => Ok(())),
                step!(AddMarker, input => (9, Some(3)), result => Ok(())),
                step!(Acknowledge, input => 4, result => 4),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Known(4), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 4, len: EligibleMarkerLength::Assumed(5), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => None),
            ],
        );
    }

    #[test]
    fn advanced_cases() {
        // A marker with a length of 0 should be immediately available:
        run_test_case(
            "zero_length_eligible",
            vec![
                step!(AddMarker, input => (0, Some(0)), result => Ok(())),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Known(0), data: None,
                    }
                )),
            ],
        );

        // When we have a fixed-size marker whose required acked marker ID lands right on the
        // current acked marker ID, it should not be eligible unless there are enough unclaimed
        // acks to actually account for it:
        run_test_case(
            "fixed_size_u64_boundary_overlap",
            vec![
                step!(AddMarker, input => (2_686_784_444_737_799_532, Some(15_759_959_628_971_752_084)), result => Ok(())),
                step!(AddMarker, input => (0, None), result => Ok(())),
                step!(AddMarker, input => (8_450_737_568, None), result => Ok(())),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Assumed(2_686_784_444_737_799_532), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => None),
                step!(Acknowledge, input => 15_759_959_628_971_752_084, result => 15_759_959_628_971_752_084),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 2_686_784_444_737_799_532, len: EligibleMarkerLength::Known(15_759_959_628_971_752_084), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => Some(
                    EligibleMarker {
                        id: 0, len: EligibleMarkerLength::Assumed(8_450_737_568), data: None,
                    }
                )),
                step!(GetNextEligibleMarker, result => None),
            ],
        );
    }

    fn run_test_case(name: &str, case: Vec<(Action, ActionResult)>) {
        let mut sut = OrderedAcknowledgements::from_acked(0u64);
        for (action, expected_result) in case {
            let actual_result = apply_action_sut(&mut sut, action);
            assert_eq!(
                expected_result, actual_result,
                "{name}: ran action {action:?} expecting result {expected_result:?}, but got result {actual_result:?} instead"
            );
        }
    }

    #[test]
    fn unclaimed_acks_overflows() {
        let actions = vec![Action::Acknowledge(u64::MAX), Action::Acknowledge(1)];

        let mut sut = OrderedAcknowledgements::<()>::from_acked(0);
        for action in actions {
            apply_action_sut(&mut sut, action);
        }
    }
}
