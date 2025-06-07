use event::Metric;

use super::dbus::{Client, Error};

const DEST: &str = "org.freedesktop.resolve1";
const PATH: &str = "/org/freedesktop/resolve1";
const INTERFACE: &str = "org.freedesktop.DBus.Properties";
const METHOD: &str = "Get";

pub async fn collect(client: &mut Client) -> Result<Vec<Metric>, Error> {
    // cache stats
    let cache_statistics = client
        .call::<[u64; 3]>(
            PATH,
            METHOD,
            DEST,
            INTERFACE,
            &["org.freedesktop.resolve1.Manager", "CacheStatistics"],
        )
        .await?;

    // transaction stats
    let transaction_statistics = client
        .call::<[u64; 2]>(
            PATH,
            METHOD,
            DEST,
            INTERFACE,
            &["org.freedesktop.resolve1.Manager", "TransactionStatistics"],
        )
        .await?;

    // dnssec stats
    let dnssec_statistics = client
        .call::<[u64; 4]>(
            PATH,
            METHOD,
            DEST,
            INTERFACE,
            &["org.freedesktop.resolve1.Manager", "DNSSECStatistics"],
        )
        .await?;

    Ok(vec![
        Metric::gauge(
            "systemd_resolved_current_cache_size",
            "Resolved Current Cache Size",
            cache_statistics[0],
        ),
        Metric::sum(
            "systemd_resolved_cache_hits_total",
            "Resolved Total Cache Hits",
            cache_statistics[1],
        ),
        Metric::sum(
            "systemd_resolved_cache_misses_total",
            "Resolved Total Cache Misses",
            cache_statistics[2],
        ),
        Metric::gauge(
            "systemd_resolved_current_transactions",
            "Resolved Current Transactions",
            transaction_statistics[0],
        ),
        Metric::sum(
            "systemd_resolved_transactions_total",
            "Resolved Total Transactions",
            transaction_statistics[1],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_secure_total",
            "Resolved Total number of DNSSEC Verdicts Secure",
            dnssec_statistics[0],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_insecure_total",
            "Resolved Total number of DNSSEC Verdicts Insecure",
            dnssec_statistics[1],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_bogus_total",
            "Resolved Total number of DNSSEC Verdicts Boguss",
            dnssec_statistics[2],
        ),
        Metric::sum(
            "systemd_resolved_dnssec_indeterminate_total",
            "Resolved Total number of DNSSEC Verdicts Indeterminat",
            dnssec_statistics[3],
        ),
    ])
}
