# PostHog Rust

Please see the main [PostHog docs](https://posthog.com/docs).

**This crate is under development**

# Quickstart

Add `posthog-rs` to your `Cargo.toml`.

```toml
[dependencies]
posthog-rs = "0.3.7"
```

```rust
let client = posthog_rs::client(env!("POSTHOG_API_KEY"));

let mut event = posthog_rs::Event::new("test", "1234");
event.insert_prop("key1", "value1").unwrap();
event.insert_prop("key2", vec!["a", "b"]).unwrap();

client.capture(event).unwrap();
```

## Automatic Batching

Events are automatically batched and sent in the background. By default, events are sent when either:
- 100 events have been queued (configurable via `flush_at`)
- 500ms have elapsed since the last flush (configurable via `flush_interval_ms`)

```rust
use posthog_rs::ClientOptionsBuilder;

let options = ClientOptionsBuilder::default()
    .api_key("your-api-key".to_string())
    .flush_at(50)           // Send after 50 events
    .flush_interval_ms(1000) // Or after 1 second
    .build()
    .unwrap();

let client = posthog_rs::client(options);
```

Remaining events are automatically flushed when the client is dropped.

## Logging

This library uses the [`log`](https://docs.rs/log) crate for logging. By default, logging is disabled. To enable it, initialize a logger in your application:

```rust
// Using env_logger (add to Cargo.toml: env_logger = "0.11")
env_logger::init();

// Or use any other logger compatible with the log crate
```

The library logs warnings when events fail to send to PostHog.
