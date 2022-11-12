# Vertex

Vertex is a fork/rewrite/rebranding of [Vector](https://github.com/vectordotdev/vector), and it is inspired 
by [OpenTelemetry](https://opentelemetry.io/).

Vertex is used for collecting and processing observability data(metrics, logs and traces). it can be used to
replace some part of your monitor infra, like `node_exporter`, `fluent-bit`, `jaeger-agent` and etc. You can
always check out the supported extensions by 
```shell
# list all supported source extension
vertex sources
```

## Concepts
### Event
Events represent the individual units of data in Vertex. An event
could be:
- log
- metric
- trace

### Components
Component is the generic term for `sources`, `transforms`, `sinks` and `extensions`. Components
ingest, transform, route events and extension Vertex. Users could compose components to craete topologies.

#### Sources
A source defines where Vertex should pull data from, or how it should receive data pushed to it.
A topology can have any number of sources, and as they ingest data they proceed to normalize it into events.
This sets the stage for easy and consistent processing of your data.

A lot [prometheus exporter](https://prometheus.io/docs/instrumenting/exporters/#exporters-and-integrations)
has already been ported.

| Name                    | Description                                                       | Metric  | Log     | Trace   |
|-------------------------|-------------------------------------------------------------------|:-------:|:-------:|:-------:|
| bind                    | Scrapes metrics from Bind server's HTTP API                       | &check; | &cross; | &cross; |
| consul                  | Scrapes metrics from consul                                       | &check; | &cross; | &cross; |
| demo_logs               | Generate logs (useful for debug)                                  | &cross; | &check; | &cross; |
| exec                    | Execute a command and capture stdout as logs                      | &cross; | &check; | &cross; |
| haproxy                 | Scrapes metrics from haproxy                                      | &check; | &cross; | &cross; |
| internal_logs           | Collect internal logs                                             | &cross; | &check; | &cross; |
| internal_metrics        | Collect internal metrics                                          | &check; | &cross; | &cross; |
| internal_traces         | Collect internal traces                                           | &cross; | &cross; | &check; |
| jaeger                  | Running as a agent/collector to collect metrics                   | &cross; | &cross; | &check; |
| kafka                   | Consume kafka messages as log events                              | &cross; | &check; | &cross; |
| kmsg                    | Read logs from /dev/kmsg                                          | &cross; | &check; | &cross; |
| kubernetes_events       | Watch kubernetes event and convert it to log event                | &cross; | &check; | &cross; |
| kubernetes_logs         | Collect logs from pod                                             | &cross; | &check; | &cross; |
| libvirt                 | Collect status from libvirt                                       | &check; | &cross; | &cross; |
| memcached               | Collect memcached stats                                           | &check; | &cross; | &cross; |
| mysqld                  | Collect various stat of mysql server                              | &check; | &cross; | &cross; |
| nginx_stub              | Collect metrics from nginx stub api                               | &check; | &cross; | &cross; |
| node                    | Collect hardware and OS metrics, just like node_exporter          | &check; | &cross; | &cross; |
| ntp                     | Collect offset, stratum, rtt and other metrics                    | &check; | &cross; | &cross; |
| nvidia_smi              | Collect Nvidia GPU status from `nvidia-smi`                       | &check; | &cross; | &cross; |
| prometheus_remote_write | Start a HTTP server to receive prometheus metrics                 | &check; | &cross; | &cross; |
| prometheus_scrape       | Scrape prometheus metrics from exporters                          | &check; | &cross; | &cross; |
| redis                   | Scrape metrics from Redis                                         | &check; | &cross; | &cross; |
| selfstat                | Collect metrics of Vertex itself, e.g. cpu, memory usage and etc. | &check; | &cross; | &cross; |
| syslog                  | Start a TCP/UDP server to receive logs                            | &cross; | &check; | &cross; |
| tail                    | Watch and collect log files                                       | &cross; | &check; | &cross; |
| zookeeper               | Collect metrics from Zookeeper ( mntr )                           | &cross; | &check; | &cross; |

#### Transforms
A transform is responsible for mutating events as they are transported by Vertex. This
might involve parsing, filtering, sampling or aggregating. You can have any number of
transforms in your pipeline, and how they are composed is up to you.

| Name                    | Description                                                       | Metric  | Log     | Trace   |
|-------------------------|-------------------------------------------------------------------|:-------:|:-------:|:-------:|
| add_tags                | Add tags for event                                                | &check; | &check; | &check; |
| coercer                 | Coerce log field to another type                                  | &cross; | &check; | &cross; |
| dedup                   | Dedup logs                                                        | &cross; | &check; | &cross; |
| enum                    | Map log fileds                                                    | &cross; | &check; | &cross; |
| filter                  | Filter out logs according field value                             | &cross; | &check; | &cross; |
| geoip                   | Add GeoIP to log field                                            | &cross; | &check; | &cross; |
| json_parser             | Parse log field and store it into another field                   | &cross; | &check; | &cross; |
| metricalize             | Consume logs and calculate value to produce metrics               | &cross; | &check; | &cross; |
| route                   | Route data to other transforms or sinks                           | &cross; | &check; | &cross; |
| sample                  | Sample data according specific log fields                         | &cross; | &check; | &cross; |
| substr                  | Act like bash's substr, eg: `${variable:4:6}`                     | &cross; | &check; | &cross; |
| throttle                | Limit the rate of events                                          | &cross; | &check; | &cross; |

#### Sinks
A sink is a destination for events. Each sink's design and transmission method is dicated by
the downstream service it interacts with.

| Name                     | Description                                                       | Metric  | Log     | Trace   |
|--------------------------|-------------------------------------------------------------------|:-------:|:-------:|:-------:|
| blackhole                | Take event and do nothing                                         | &check; | &check; | &check; |
| console                  | Print data to stdout or stderr                                    | &check; | &check; | &check; |
| jaeger                   | Send traces to jaeger agent or collector                          | &cross; | &cross; | &check; |
| kafka                    | Send events to kafka                                              | &cross; | &check; | &cross; |
| loki                     | Send logs to Loki                                                 | &cross; | &check; | &cross; |
| prometheus_exporter      | Start a HTTP server and expose metrics                            | &check; | &cross; | &cross; |
| prometheus_remote_write  | Push metrics to Prometheus                                        | &check; | &cross; | &cross; |
| socket                   | Push data                                                         | &check; | &check; | &check; |

#### Extensions
An extension is

| Name                | Description                                                            |
|---------------------|------------------------------------------------------------------------|
| healthcheck         | Start an HTTP server, and return 200 to represent Vertex is health     |
| pprof               | Start an HTTP server to help user profile Vertex, implement by [pprof-rs](https://github.com/tikv/pprof-rs) |

## Build
Note: Only x86_64-unknown-linux-gnu(or musl) is supported.

### Install additional dependencies
- Rust (a recent stable version, 1.60 or higher). To install Rust, we recommend using rustup.
- make
- protobuf
- docker
- [cross](https://github.com/cross-rs/cross)

### Build with make

```shell
make build
```

build for x86_64-unknown-linux-musl
```shell
make x86_64-unknown-linux-musl
```

## Configuration
Let's see this `node_exporter` alternative configuration

```yaml
sources:
  selfstat:
    type: selfstat
  node:
    type: node_metrics
    # default value is 15s
    # interval: 15s

transforms:
  add_hosts:
    type: add_tags
    tags:
      foo: bar
      host: ${HOSTNAME} # environment variables
    inputs:
      - selfstat
      - node

sinks:
  prom:
    type: prometheus_exporter
    # default listent to 0.0.0.0:9100
    endpoint: 0.0.0.0:9100
    inputs:
      - add_hosts
```

There are some keywords
- `type` is used to represent component type
- `inputs` is an array used to build the Topology(DAG)

### Config example
There are dozens of components, configuration could be very difficult. Therefore,
`vertex sources|transforms|sinks|extensions [name]` could be very help

For example,
```shell
$ ./target/release/vertex sources node
# The interval between scrapes.
#
# interval: 15s

# Proc path
#
proc_path: /proc

# Sys path
#
sys_path: /sys
```

There are more example configurations under `examples/config` to help you know what's Vertex capable of.
