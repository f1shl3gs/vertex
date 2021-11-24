# Remote Configuration

First we need a solution for `Remote Configuration`, which will save us lots of 
time when we change the configuration.

# Configuration
## Goal
Allow remote configuration of Vertex by feeding the configuration to the Vertex 
from a remote configuration source. The source of the remote configuration must 
be possible to specify in the Vertex's local config file via a pluggable component
that can be implemented by any third-party developers.

## Summary
We need to define a remote configuration interface(internal API) between Vertex 
config and remote configuration implementations. The interface should preferably 
have a "watcher" style, where the implementation notifies the Vertex core about  
the availability of a new configuration(as opposed to Vertex core periodically 
polling the implementation asking for a new configuration).

## Operation
The Vertex will initialize the Remote configuration implementation and will provide 
attributes that identify the Vertex, then will wait to be notified about the 
availability of a new config. The implementation may use these identifying attributes
to fetch the configuration that is applicable to this particular Vertex.

The Vertex will identify itself using the following attributes:
- Static attributes defined at build time, such as Vertex version, version of OS 
    it is built for, commit hash, etc.
- Dynamic attributes that the Vertex will auto-detect at runtime, such as the OS 
    version it runs on, the machine id it runs on(if available), etc.
- User-defined attributes that are specified manually by the end user in the local 
    config file used by the Vertex(such as for example "env=prod").
- Vertex's unique instance id, specified in "service.instance.id" attribute. The 
    Vertex will attempt to obtain this from a persistent ID source(such as machine 
    UUID), falling back to an ephemeral generated UID.

On startup the Vertex with an enabled remote configuration option should wait for the remote
configuration to arrive before the Vertex's regular operation begins. This behavior may be 
configurable locally(e.g. how long to wait for).

After the Vertex receives a remote config it will attempt to reconfigure itself. If the
reconfiguration fails the Vertex will revert to the last known good config.

The reconfiguration requires graceful shutdown, reconfiguration and restart of the Vertex.

## Unique Instance ID
We will try to fetch persistent machine id when it is available using a
library like this. When persistent machine id is not available the Vertex 
will generate random ephemeral UUID for it's UID. Ephemeral UID is not very 
useful for remote long-term config purposes but is still useful for uniquely
identifying the Vertex at least during one session. This allows to tie status
reports with the particular Vertex instance and show reported effective 
config or config errors in the UI.

In the future we may add an ability for the Vertex to inform the backend the 
UID is ephemeral so that the UI warns the user not to use it to create a
partial config.

We may also add an ability to detect duplicate UIDs in the future, if we are
not confident that the persistent or ephemeral UIDs are unique enough.

## Security
Remotely controlled configuration is a security risk. Via remote configuration 
the Vertex may be compelled to collect data and send to a destination. Vertex 
today is capable of collecting data both passively by accepting it and actively 
by scraping metrics from locally and remotely running systems.

In order to reduce this risk we make remote configuration capability disabled
by default. It has to be explicitly enabled by the user using a setting in
local configuration file.

In the future we may have more dangerous capabilities, such as ability to 
execute external processes for metric/log collection.

## Push vs Pull
This proposal suggests the Vertex to be notified when a new config is available. 
We could instead design the internal Remote Config API in a way that requires 
Vertex to poll the remote source for config changes.

This can simplify implementation but increase the time that the configuration 
changes become effective.

## Heartbeat
With this implement we should also implement heartbeat too, then the UI can show
us how many (healthy or unhealthy) vertex connected. If the instance haven't sent 
heartbeat for 5m(just an example), then it should be marked as instance lost.   

Note: `The config server might get performance issues when there are lots of vertex
instances connect`

## Route
With the attributes provided by vertex, we can implement something like a route
to delivery different config for different Vertex. For example, if the attributes
contains "env=prod", then we response the "prod" config, etc. 

## Plan of Attack
