# Jaeger
This crate provides basic `structs` and `convert` to help building Jaeger source and sink

# Protocol
Original protobuf files provided by `jaeger` include `gogo`(popular in golang applications), 
so we have to remove it to make sure `prost` can work.

## WARN
Modifying thrift generated files is not allowed, but still doing this for
implement `deserialize_compact_batch`.