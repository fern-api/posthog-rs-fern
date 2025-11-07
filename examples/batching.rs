// PostHog Rust SDK - Batching Example
//
// This example demonstrates automatic event batching with the PostHog Rust SDK.
// Events are automatically batched and sent in the background when either:
// - flush_at events have been queued (default: 100)
// - flush_interval_ms time has elapsed (default: 500ms)
//
// Setup:
// Set POSTHOG_API_KEY environment variable:
//    export POSTHOG_API_KEY="your-project-api-key"
//
// Run:
//    cargo run --example batching
//
// Note: For self-hosted instances, use ClientOptionsBuilder:
//    .api_endpoint("https://your-posthog.com".to_string())

use dotenv::dotenv;
use posthog_rs::{client, ClientOptionsBuilder, Event};
use std::env;

#[cfg(feature = "async-client")]
#[tokio::main]
async fn main() {
    run_async().await;
}

#[cfg(not(feature = "async-client"))]
fn main() {
    run_blocking();
}

#[cfg(feature = "async-client")]
async fn run_async() {
    println!("PostHog Rust SDK - Async Batching Example\n");

    // Load .env file if it exists
    dotenv().ok();

    // Get API key from environment
    let api_key = env::var("POSTHOG_API_KEY").expect(
        "POSTHOG_API_KEY must be set. Example: export POSTHOG_API_KEY=\"phc_...\"",
    );

    println!("=== Example 1: Default Batching Configuration ===");
    println!("Using default settings: flush_at=100, flush_interval_ms=500\n");

    {
        let client = client(api_key.as_str()).await;

        // Send a few events with default batching
        for i in 0..5 {
            let distinct_id = format!("user_{}", i);
            let mut event = Event::new("example_event", &distinct_id);
            event
                .insert_prop("example_type", "default_config")
                .unwrap();
            event.insert_prop("event_number", i).unwrap();

            client.capture(event).await.unwrap();
            println!("Queued event {} for user_{}", i, i);
        }

        println!("\nEvents will be sent after 500ms or when 100 events are queued");
        println!("Waiting for automatic flush...");
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        println!("✅ Events sent!");
    }

    println!("\n=== Example 2: Custom Batching Configuration ===");
    println!("Using custom settings: flush_at=3, flush_interval_ms=2000\n");

    let options = ClientOptionsBuilder::default()
        .api_key(api_key)
        .flush_at(3) // Send batch after 3 events
        .flush_interval_ms(2000) // Or after 2 seconds
        .build()
        .unwrap();

    let client2 = client(options).await;

    // Send events that will trigger batching at flush_at threshold
    for i in 0..5 {
        let distinct_id = format!("user_custom_{}", i);
        let mut event = Event::new("example_event_custom", &distinct_id);
        event
            .insert_prop("example_type", "custom_config")
            .unwrap();
        event.insert_prop("event_number", i).unwrap();

        client2.capture(event).await.unwrap();
        println!("Queued event {}", i);

        if (i + 1) % 3 == 0 {
            println!("  → Batch sent! (reached flush_at=3)");
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    println!("\nWaiting for final events to flush...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    println!("✅ All events sent!");

    println!("\n=== Graceful Shutdown ===");
    println!("When the client is dropped, remaining events are automatically flushed.");
    println!("Check your PostHog dashboard to verify events arrived!");
}

#[cfg(not(feature = "async-client"))]
fn run_blocking() {
    use std::thread;
    use std::time::Duration;

    println!("PostHog Rust SDK - Blocking Batching Example\n");

    // Load .env file if it exists
    dotenv().ok();

    // Get API key from environment
    let api_key = env::var("POSTHOG_API_KEY").expect(
        "POSTHOG_API_KEY must be set. Example: export POSTHOG_API_KEY=\"phc_...\"",
    );

    println!("=== Example 1: Default Batching Configuration ===");
    println!("Using default settings: flush_at=100, flush_interval_ms=500\n");

    {
        let client = client(api_key.as_str());

        // Send a few events with default batching
        for i in 0..5 {
            let distinct_id = format!("user_{}", i);
            let mut event = Event::new("example_event", &distinct_id);
            event
                .insert_prop("example_type", "default_config")
                .unwrap();
            event.insert_prop("event_number", i).unwrap();

            client.capture(event).unwrap();
            println!("Queued event {} for user_{}", i, i);
        }

        println!("\nEvents will be sent after 500ms or when 100 events are queued");
        println!("Waiting for automatic flush...");
        thread::sleep(Duration::from_millis(1000));
        println!("✅ Events sent!");
    }

    println!("\n=== Example 2: Custom Batching Configuration ===");
    println!("Using custom settings: flush_at=3, flush_interval_ms=2000\n");

    let options = ClientOptionsBuilder::default()
        .api_key(api_key)
        .flush_at(3) // Send batch after 3 events
        .flush_interval_ms(2000) // Or after 2 seconds
        .build()
        .unwrap();

    let client2 = client(options);

    // Send events that will trigger batching at flush_at threshold
    for i in 0..5 {
        let distinct_id = format!("user_custom_{}", i);
        let mut event = Event::new("example_event_custom", &distinct_id);
        event
            .insert_prop("example_type", "custom_config")
            .unwrap();
        event.insert_prop("event_number", i).unwrap();

        client2.capture(event).unwrap();
        println!("Queued event {}", i);

        if (i + 1) % 3 == 0 {
            println!("  → Batch sent! (reached flush_at=3)");
            thread::sleep(Duration::from_millis(100));
        }
    }

    println!("\nWaiting for final events to flush...");
    thread::sleep(Duration::from_secs(3));
    println!("✅ All events sent!");

    println!("\n=== Graceful Shutdown ===");
    println!("When the client is dropped, remaining events are automatically flushed.");
    println!("Check your PostHog dashboard to verify events arrived!");
}
