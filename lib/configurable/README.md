# Configurable

```rust
#[configurable_component(
    sink,
    name = "prometheus"
)]
struct PrometheusConfig {
    #[configurable(
        required,
        description = "Address the http server will listen to",
        examples = "0.0.0.0:9000",
        default = "127.0.0.1:9000"
    )]
    listen: String,
}
```
