# Configuration & Service Discovery

First we need a solution for `Remote Configuration`, which will save us lots of time when we change the configruation.

# Configuration
## Goal
Allow remote configuration of Vertex by feeding the configuration to the Vertex from a remote configuration source.
The source of the remote configuration must be possible to specify in the Vertex's local config file via a 
pluggable component that can be implemented by any third-party developers.

## Summary

## Push vs Pull
This proposal suggests the Vertex to be notified when a new config is available. We could instead design 
the internal Remote Config API in a way that requires Vertex to poll the remote source for config changes.

This can simplify implementation but increase the time that the configuration changes become effective.

# Service Discovery
There is no such standard service discovery to connect to like `Consul`, it would be cool to
have Vertex connect to that and automatically start collecting data for service that Vertex support.

So when a new Server/Pod/MySQL comes on, Vertex will automatically start collecting data from it.

`OpenTelemetry` already implement a handy [discovery mechanism](https://github.com/open-telemetry/opentelemetry-collector-contrib/tree/main/receiver/receivercreator),
and it is very promising. 

This implement has two main parts:
- Discovery targets/services from ports or kubernetes
- Create receiver(source is call in vertex) dynamically. So the source should be very simple.

```yaml
extensions:
  k8s: {} # some config

sources:
  mysqls:
    type: creator
    discovery: 
      - k8s
    # just a demo, it will change in the futures
    templates:
      "{{ label.service }}": 
        type: mysql
        host: {{ annotations.ip_address }}
        port: {{ annotations.port }}

sinks:
  stdout:
    type: stdout
    inputs: mysqls
```