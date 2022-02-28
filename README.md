# Vertex

Vertex is a fork/rewrite/rebranding of [Vector](https://github.com/vectordotdev/vector), and it is inspired by [OpenTelemetry](https://opentelemetry.io/).

## Sources

### Node
#### todo:
- vmstat_numa: https://github.com/prometheus/node_exporter/pull/1951
- protocols: https://github.com/prometheus/node_exporter/pull/1921
- netdev: implement with netlink? https://github.com/prometheus/node_exporter/pull/2074
- mountstats: disabled by default. Exposes filesystem statistics from `/proc/self/mountstats`. Exposes detailed NFS client statistics.
- processes: disabled by default, Exposes aggregate process statistics from
- network_route: Exposes the routing table as metrics, which can be implemented by netlink

## TLS
At the beginning, Rustls is our first choice, but there is something we don't expect
(https://github.com/briansmith/webpki/pull/120), which is necessary for self-sign certs.