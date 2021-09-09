/// Exposes selected adjtimex(2) system call stats.

use crate::{
    gauge_metric,
    sum_metric,
    event::{Metric, MetricValue},
};
use crate::sources::node::errors::{Error, ErrorContext};

// The system clock is not synchronized to a reliable
// server (TIME_ERROR)
const TIME_ERROR: i32 = 5;

// The timex.Status time resolution bit (STA_NANO),
// 0 = microsecond, 1 = nanoseconds
const STA_NANO: i32 = 0x2000;

// 1 second in
const NANOSECONDS: f64 = 1000000000.0;
const MICROSECONDS: f64 = 1000000.0;

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let (tx, status) = adjtimex()
        .context("syscall adjtimex failed")?;

    let sync_status = match status {
        TIME_ERROR => 0,
        _ => 1
    };

    let divisor = match tx.status & STA_NANO {
        0 => MICROSECONDS,
        _ => NANOSECONDS,
    } as f64;
    // See NOTES in adjtimex(2).
    const PPM16FRAC: f64 = 1000000.0 * 65536.0;

    Ok(vec![
        gauge_metric!(
            "node_timex_sync_status",
            "Is clock synchronized to a reliable server (1 = yes, 0 = no).",
            sync_status as f64
        ),
        gauge_metric!(
            "node_timex_offset_seconds",
            "Time offset in between local system and reference clock.",
            tx.offset as f64 / divisor
        ),
        gauge_metric!(
            "node_timex_frequency_adjustment_ratio",
            "Local clock frequency adjustment.",
            tx.freq as f64 / PPM16FRAC
        ),
        gauge_metric!(
            "node_timex_maxerror_seconds",
            "Maximum error in seconds.",
            tx.maxerror as f64 / MICROSECONDS
        ),
        gauge_metric!(
            "node_timex_estimated_error_seconds",
            "Estimated error in seconds.",
            tx.esterror as f64 / MICROSECONDS
        ),
        gauge_metric!(
            "node_timex_status",
            "Value of the status array bits.",
            tx.status as f64
        ),
        gauge_metric!(
            "node_timex_loop_time_constant",
            "Phase-locked loop time constant.",
            tx.constant as f64
        ),
        gauge_metric!(
            "node_timex_tick_seconds",
            "Seconds between clock ticks.",
            tx.tick as f64 / MICROSECONDS
        ),
        gauge_metric!(
            "node_timex_pps_frequency_hertz",
            "Pulse per second frequency.",
            tx.ppsfreq as f64 / PPM16FRAC
        ),
        gauge_metric!(
            "node_timex_pps_jitter_seconds",
            "Pulse per second jitter.",
            tx.jitter as f64 / divisor
        ),
        gauge_metric!(
            "node_timex_pps_shift_seconds",
            "Pulse per second interval duration.",
            tx.shift as f64
        ),
        gauge_metric!(
            "node_timex_pps_stability_hertz",
            "Pulse per second stability, average of recent frequency changes.",
            tx.stabil as f64
        ),
        gauge_metric!(
            "node_timex_pps_jitter_total",
            "Pulse per second count of jitter limit exceeded events.",
            tx.jitcnt as f64
        ),
        sum_metric!(
            "node_timex_pps_calibration_total",
            "Pulse per second count of calibration intervals.",
            tx.calcnt as f64
        ),
        sum_metric!(
            "node_timex_pps_error_total",
            "Pulse per second count of calibration errors.",
            tx.errcnt as f64
        ),
        sum_metric!(
            "node_timex_pps_stability_exceeded_total",
            "Pulse per second count of stability limit exceeded events.",
            tx.stbcnt as f64
        ),
        gauge_metric!(
            "node_timex_tai_offset_seconds",
            "International Atomic Time (TAI) offset.",
            tx.tai as f64
        ),
    ])
}

#[derive(Default)]
struct Timex {
    // not all fields listed
    pub modes: u32,
    pub offset: i64,
    pub freq: i64,
    pub maxerror: i64,
    pub esterror: i64,
    pub status: i32,
    pub constant: i64,
    pub precision: i64,
    pub tolerance: i64,
    pub tick: i64,
    pub ppsfreq: i64,
    pub jitter: i64,
    pub shift: i32,
    pub stabil: i64,
    pub jitcnt: i64,
    pub calcnt: i64,
    pub errcnt: i64,
    pub stbcnt: i64,
    pub tai: i32,
}

impl From<libc::timex> for Timex {
    fn from(tx: libc::timex) -> Self {
        Self {
            modes: tx.modes as u32,
            offset: tx.offset as i64,
            freq: tx.freq,
            maxerror: tx.maxerror,
            esterror: tx.esterror,
            status: tx.status,
            constant: tx.constant,
            precision: tx.precision,
            tolerance: tx.tolerance,
            tick: tx.tick,
            ppsfreq: tx.ppsfreq,
            jitter: tx.jitter,
            shift: tx.shift,
            stabil: tx.stabil,
            jitcnt: tx.jitcnt,
            calcnt: tx.calcnt,
            errcnt: tx.errcnt,
            stbcnt: tx.stbcnt,
            tai: tx.tai,
        }
    }
}

fn adjtimex() -> Result<(Timex, i32), Error> {
    let result = unsafe {
        let mut tx = std::mem::zeroed();
        let r = libc::adjtimex(&mut tx);
        if r != 0 {
            return Err(Error::from(std::io::Error::last_os_error()));
        }

        (Timex::from(tx), r as i32)
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjtimex() {
        let ctx = Timex::default();

        unsafe {
            let mut tx = std::mem::zeroed();
            if libc::adjtimex(&mut tx) != 0 {
                panic!("syscall adjtimex failed, {}", std::io::Error::last_os_error());
            }
        }
    }
}