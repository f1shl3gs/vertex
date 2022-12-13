# Vertex

Vertex is a fork/rewrite/rebranding of [Vector](https://github.com/vectordotdev/vector), and it is inspired 
by [OpenTelemetry](https://opentelemetry.io/).

Vertex is used for collecting and processing observability data(metrics, logs and traces). it can be used to
replace some part of your monitor infra, like `node_exporter`, `fluent-bit`, `jaeger-agent` and etc.

## Features
- Hot reload from files or HTTP endpoint
  - File watch needs `inotify`, so only Linux supported.
  - HTTP support get or get-and-watch style(like Kubernetes), WebSocket is not supported.

## Build
Note: Only x86_64-unknown-linux-gnu(or musl) is supported.

```shell
make build
```

## Configuration

Show config example for Consul
```shell
vertex sources consul
# HTTP/HTTPS endpoint to Consul server.
endpoints:
- http://localhost:8500

# The interval between scrapes.
#
# interval: 15s

# Configures the TLS options for outgoing connections.
# tls:
#  # Absolute path to an additional CA certificate file, in DER or PEM
#  # format(X.509), or an inline CA certificate in PEM format
#  ca_file: /path/to/certificate_authority.crt
#  
#  # Absolute path to a certificate file used to identify this connection,
#  # in DER or PEM format (X.509) or PKCS#12, or an inline certificate in
#  # PEM format. If this is set and is not a PKCS#12 archive, "key_file"
#  # must also be set.
#  crt_file: /path/to/host_certificate.crt
#  
#  # Absolute path to a private key file used to identify this connection,
#  # in DER or PEM format (PKCS#8), or an inline private key in PEM format.
#  # If this is set, "crt_file" must also be set.
#  key_file: /path/to/host_certificate.key
#  
#  # Pass phrase used to unlock the encrypted key file. This has no effect
#  # unless "key_file" is set.
#  key_pass: some_password
#  
#  # If "true", Vertex will validate the configured remote host name against
#  # the remote host's TLS certificate. Do NOT set this to false unless you
#  # understand the risks of not verifying the remote hostname.
#  verify_hostname: true
```

There are more example configurations under `examples/config` to help you know what's Vertex capable of.

## Sources
A source defines how Vertex should collect data from(push or pull), 
a lot [prometheus exporter](https://prometheus.io/docs/instrumenting/exporters/#exporters-and-integrations)
has already been ported.

| Name                    | Description                                                       | Status  |
|-------------------------|-------------------------------------------------------------------|:-------:|
| bind                    | Scrapes metrics from Bind server's HTTP API                       | &check; |
| consul                  | Scrapes metrics from consul                                       | &check; |
| demo_logs               | Generate logs (useful for debug)                                  | &check; |
| exec                    | Execute a command and capture stdout as logs                      | &check; |
| haproxy                 | Scrapes metrics from haproxy                                      | &check; |
| internal_logs           | Collect internal logs                                             | &check; |
| internal_metrics        | Collect internal metrics                                          | &check; |
| internal_traces         | Collect internal traces                                           | &check; |
| jaeger                  | Running as a agent/collector to collect metrics                   | &check; |
| kafka                   | Consume kafka messages as log events                              | &check; |
| kmsg                    | Read logs from /dev/kmsg                                          | &check; |
| kubernetes_events       | Watch kubernetes event and convert it to log event                | &check; |
| kubernetes_logs         | Collect logs from pod                                             | &check; |
| libvirt                 | Collect status from libvirt                                       | &check; |
| memcached               | Collect memcached stats                                           | &check; |
| mongodb                 |                                                                   | &cross; |
| mysqld                  | Collect various stat of mysql server                              | &check; |
| nginx_stub              | Collect metrics from nginx stub api                               | &check; |
| node                    | Collect hardware and OS metrics, just like node_exporter          | &check; |
| ntp                     | Collect offset, stratum, rtt and other metrics                    | &check; |
| nvidia_smi              | Collect Nvidia GPU status from `nvidia-smi`                       | &check; |
| prometheus_remote_write | Start a HTTP server to receive prometheus metrics                 | &check; |
| prometheus_scrape       | Scrape prometheus metrics from exporters                          | &check; |
| redis                   | Scrape metrics from Redis                                         | &check; |
| selfstat                | Collect metrics of Vertex itself, e.g. cpu, memory usage and etc. | &check; |
| syslog                  | Start a TCP/UDP server to receive logs                            | &check; |
| tail                    | Watch and collect log files                                       | &check; |
| zookeeper               | Collect metrics from Zookeeper ( mntr )                           | &check; |

### Node
#### todo:
- vmstat_numa: https://github.com/prometheus/node_exporter/pull/1951
- protocols: https://github.com/prometheus/node_exporter/pull/1921
- netdev: implement with netlink? https://github.com/prometheus/node_exporter/pull/2074
- mountstats: disabled by default. Exposes filesystem statistics from `/proc/self/mountstats`. 
  Exposes detailed NFS client statistics.
- processes: disabled by default, Exposes aggregate process statistics from
- network_route: Exposes the routing table as metrics, which can be implemented by netlink

## TLS
At the beginning, Rustls is our first choice, but there is something we don't expect
(https://github.com/briansmith/webpki/pull/120), which is necessary for self-sign certs.