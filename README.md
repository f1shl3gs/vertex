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

#### Transforms
A transform is responsible for mutating events as they are transported by Vertex. This
might involve parsing, filtering, sampling or aggregating. You can have any number of
transforms in your pipeline, and how they are composed is up to you.



#### Sinks
A sink is a destination for events. Each sink's design and transmission method is dicated by
the downstream service it interacts with.

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
