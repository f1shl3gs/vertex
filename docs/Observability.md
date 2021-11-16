# Observability

## Pain

### Self Describe
For now, vertex do this as `Vector` do, but it is not always good enough.
```rust
#[derive(Debug)]
pub struct GotHttpError<'a> {
    pub error: &'a Error,
    pub roundtrip: Duration,
}

impl<'a> InternalEvent for GotHttpError<'a> {
    fn emit_logs(&self) {
        debug!(
            message = "HTTP error",
            error = %self.error
        );
    }

    fn emit_metrics(&self) {
        counter!("http_client_errors_total", 1, "error" => self.error.to_string());
        histogram!("http_client_rtt_seconds", self.roundtrip);
        histogram!("http_client_error_rtt_seconds", self.roundtrip, "error" => self.error.to_string());
    }
}
```

Like this, we can get some basic information indeed, but `why vertex send this request?`, `which component send this?` and
`request for what?`. With the logs we emitted, it's hard to figure out which step we fail, so we probably add a log like this
to help us understand what's going on, which is duplicated with the log we emitted in the event.
```rust
// `tracing` will tell record the mod call this, so we can figure out which component trying to send request.  
warn!(
    message = "Request xxx failed",
    %err
);
```
There is a simple solution, add logs and metrics more specific. e.g.
```rust
warn!(
    message = "Request xxx failed",
    %err
);
counter!("component_request_resource_error_total", 1);
```
At this situation, `InternalEvent` is totally unnecessary, this must be considered again.


### Definition
Some events contain the struct other crate defined, so define those events without introduce 
dependencies in `lib/internal` is impossible. So putting common and dep-agnostic event is `lib/internal`,
others put in their own mod?

### Reference
#### Performance
The macros of `metrics` is really convenient, but the performance might not that good, because it will
hash every time `counter!()`, `gauge!()`, `histogram` called, maybe we can skip this and increase it by reference just like 
prometheus does.
```rust
// Something like this
const HTTP_REQUESTS : Counter = Counter::new("http_request_total");

fn foo() {
    // ...
    HTTP_REQUESTS.increase();
    // ...
}
```
#### Register & Deregister
Metric can be registered by `new()`, and `Drop` will de-register it. It looks just fine, but codes might
not be clean and simple. All components of Vertex are created, modified or removed dynamically, so `const`
must not be used. put those in `SourceConfig` is not a good practice definitely, register metrics when 
the async function start is great(if the component is not very complex).


## Drawbacks

## Outstanding Questions
- Some metrics are shared across all mods, type error shall not happen. and this is hard to detect.
- What if some error event is fine, how to handle this?

## POA

