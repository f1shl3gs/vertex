use crate::disk::ledger::Ledger;
use crate::Acker;
use std::sync::Arc;

pub(super) fn create_disk_acker(ledger: Arc<Ledger>) -> Acker {
    Acker::segmented(move |amount: usize| {
        ledger.increment_pending_acks(amount);
        ledger.notify_writer_waiters();
    })
}
