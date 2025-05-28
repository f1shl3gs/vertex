use event::Metric;

use super::dbus::{Client, Error};

const DEST: &str = "org.freedesktop.resolve1";
const PATH: &str = "/org/freedesktop/resolve1";
const INTERFACE: &str = "org.freedesktop.DBus.Properties";
const METHOD: &str = "Get";

pub async fn collect(client: &mut Client) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::with_capacity(9);

    // cache stats
    let statistics = client
        .call::<[u64; 3]>(
            PATH,
            METHOD,
            DEST,
            INTERFACE,
            &["org.freedesktop.resolve1.Manager", "CacheStatistics"],
        )
        .await?;
    metrics.extend([
        Metric::gauge(
            "systemd_resolved_current_cache_size",
            "Resolved Current Cache Size",
            statistics[0],
        ),
        Metric::sum(
            "systemd_resolved_cache_hits_total",
            "Resolved Total Cache Hits",
            statistics[1],
        ),
        Metric::sum(
            "systemd_resolved_cache_misses_total",
            "Resolved Total Cache Misses",
            statistics[2],
        ),
    ]);

    // transaction stats
    let statistics = client
        .call::<[u64; 2]>(
            PATH,
            METHOD,
            DEST,
            INTERFACE,
            &["org.freedesktop.resolve1.Manager", "TransactionStatistics"],
        )
        .await?;
    metrics.extend([
        Metric::gauge(
            "systemd_resolved_current_transactions",
            "Resolved Current Transactions",
            statistics[0],
        ),
        Metric::sum(
            "systemd_resolved_transactions_total",
            "Resolved Total Transactions",
            statistics[1],
        ),
    ]);

    // dnssec stats
    let statistics = client
        .call::<[u64; 4]>(
            PATH,
            METHOD,
            DEST,
            INTERFACE,
            &["org.freedesktop.resolve1.Manager", "DNSSECStatistics"],
        )
        .await?;
    metrics.extend([
        Metric::sum(
            "systemd_resolved_dnssec_secure_total",
            "Resolved Total number of DNSSEC Verdicts Secure",
            statistics[0],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_insecure_total",
            "Resolved Total number of DNSSEC Verdicts Insecure",
            statistics[1],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_bogus_total",
            "Resolved Total number of DNSSEC Verdicts Boguss",
            statistics[2],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_indeterminate_total",
            "Resolved Total number of DNSSEC Verdicts Indeterminat",
            statistics[3],
        ),
    ]);

    Ok(metrics)
}
