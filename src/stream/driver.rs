use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::num::NonZeroUsize;

/// Newtype wrapper around sequence numbers to enforce misuse resistance.
#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
struct SequenceNumber(u64);

impl SequenceNumber {
    /// Gets the actual integer value of this sequence number
    ///
    /// This can be used trivially for correlating a given `SequenceNumber`
    /// in logs/metrics/tracings
    fn id(&self) -> u64 {
        self.0
    }
}

/// An out-of-order acknowledgement waiting to become valid
struct PendingAcknowledgement {
    seq_num: SequenceNumber,
    ack_size: usize,
}

impl PartialEq for PendingAcknowledgement {
    fn eq(&self, other: &Self) -> bool {
        self.seq_num == other.seq_num
    }
}

impl Eq for PendingAcknowledgement {}

impl PartialOrd for PendingAcknowledgement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering so that in a `BinaryHeap`, the lowest sequence number
        // is the highest priority.
        Some(other.seq_num.cmp(&self.seq_num))
    }
}

impl Ord for PendingAcknowledgement {
    fn cmp(&self, other: &Self) -> Ordering {
        other.partial_cmp(self)
            .expect("PendingAcknowledgement should always return a valid comparison")
    }
}

#[derive(Default)]
struct AcknowledgementTracker {
    out_of_order: BinaryHeap<PendingAcknowledgement>,
    seq_head: u64,
    seq_tail: u64,
    ack_depth: usize,
}

impl AcknowledgementTracker {
    /// Acquires the next available sequence number
    fn get_next_seq_num(&mut self) -> SequenceNumber {
        let seq_num = self.seq_head;
        self.seq_head += 1;
        SequenceNumber(seq_num)
    }

    /// Marks the given sequence number as complete
    fn mark_seq_num_complete(&mut self, seq_num: SequenceNumber, ack_size: usize) {
        if seq_num.0 == self.seq_tail {
            self.ack_depth += ack_size;
            self.seq_tail += 1;
        } else {
            self.out_of_order
                .push(PendingAcknowledgement { seq_num, ack_size })
        }
    }

    /// Consumes the current acknowledgement "depth"
    ///
    /// When a sequence number is marked as complete, we either update our tail pointer
    /// if the acknowledgement is "in order" -- essentially, it was the very next sequence
    /// number we expected to see -- or store it for later if it's out-of-order
    ///
    /// In this method, we see if any of the out-of-order sequence numbers can now be
    /// applied: may be 9 sequence numbers were marked complete, but one number that
    /// came before all of them was still pending, so they had to be stored in the
    /// out-of-order list to be checked later. This is where we check them.
    ///
    /// For any sequence number -- whether it completed in order or had to be applied fr.m
    /// the out-of-order list -- there is an associated acknowledge "depth", which can be
    /// though of the amount of items the sequence is acknowledgement as complete.
    ///
    /// We accumulate that amount for every sequence number between calls to `consume_ack_depth`.
    /// Thus, a fresh instance of `AcknowledgementTracker` has an acknowledgement depth of 0. If
    /// we create five sequence numbers, and mark them all complete with an acknowledge meant of
    /// 10. our depth would then be 50. Calling this method would return `Some(50)`, and if this
    /// method was called again immediately after, it would return `None`.
    fn consume_ack_depth(&mut self) -> Option<NonZeroUsize> {
        // Drain any out-of-order acknowledgements that can now be ordered correctly.
        while let Some(ack) = self.out_of_order.peek() {
            if ack.seq_num.0 == self.seq_tail {
                let PendingAcknowledgement { ack_size, .. } = self.out_of_order
                    .pop()
                    .expect("should not be here unless self.out_of_order is non-empty");

                self.ack_depth += ack_size;
                self.seq_tail += 1;
            } else {
                break;
            }
        }

        match self.ack_depth {
            0 => None,
            n => {
                self.ack_depth = 0;
                NonZeroUsize::new(n)
            }
        }
    }
}

// TODO: