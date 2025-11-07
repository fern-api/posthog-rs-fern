#[cfg(feature = "async-client")]
mod async_batching_tests {
    use httpmock::prelude::*;
    use posthog_rs::{client, ClientOptionsBuilder, Event};
    use std::time::Duration;

    #[tokio::test]
    async fn test_batching_flush_at_threshold() {
        // Test that events are batched and sent when reaching flush_at limit
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(5) // Flush after 5 events
            .flush_interval_ms(10000) // Long timeout so we test flush_at, not timeout
            .build()
            .unwrap();

        let client = client(options).await;

        // Send exactly 5 events
        for i in 0..5 {
            let distinct_id = format!("user_{}", i);
            let event = Event::new("test_event", &distinct_id);
            client.capture(event).await.unwrap();
        }

        // Give worker time to process and send batch
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify exactly 1 batch was sent
        mock.assert_hits(1);
    }

    #[tokio::test]
    async fn test_batching_flush_interval_timeout() {
        // Test that events are flushed after flush_interval timeout
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(100) // High threshold so we test timeout, not flush_at
            .flush_interval_ms(100) // Short timeout for test
            .build()
            .unwrap();

        let client = client(options).await;

        // Send only 3 events (below flush_at)
        for i in 0..3 {
            let distinct_id = format!("user_{}", i);
            let event = Event::new("test_event", &distinct_id);
            client.capture(event).await.unwrap();
        }

        // Wait for flush_interval to trigger
        tokio::time::sleep(Duration::from_millis(250)).await;

        // Verify 1 batch was sent
        mock.assert_hits(1);
    }

    #[tokio::test]
    async fn test_graceful_shutdown_flushes_remaining_events() {
        // Test that Drop implementation flushes remaining events
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        {
            let options = ClientOptionsBuilder::default()
                .api_key("test_key".to_string())
                .api_endpoint(format!("{}/batch/", server.base_url()))
                .flush_at(100) // High threshold
                .flush_interval_ms(10000) // Long timeout
                .build()
                .unwrap();

            let client = client(options).await;

            // Send 3 events (below both thresholds)
            for i in 0..3 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                client.capture(event).await.unwrap();
            }

            // Client drops here, should flush remaining events
        }

        // Give worker time to flush on shutdown
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify the 3 events were flushed
        mock.assert_hits(1);
    }

    #[tokio::test]
    async fn test_multiple_batches() {
        // Test that worker continues processing multiple batches
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(5)
            .flush_interval_ms(10000)
            .build()
            .unwrap();

        let client = client(options).await;

        // Send 15 events (should trigger 3 batches of 5)
        for i in 0..15 {
            let distinct_id = format!("user_{}", i);
            let event = Event::new("test_event", &distinct_id);
            client.capture(event).await.unwrap();
        }

        // Give worker time to process all batches
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify we received 3 batches
        mock.assert_hits(3);
    }

    #[tokio::test]
    async fn test_capture_batch_method() {
        // Test that capture_batch queues all events
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(10)
            .flush_interval_ms(10000)
            .build()
            .unwrap();

        let client = client(options).await;

        // Use capture_batch to send 10 events at once
        let events: Vec<_> = (0..10)
            .map(|i| {
                let distinct_id = format!("user_{}", i);
                Event::new("test_event", &distinct_id)
            })
            .collect();

        client.capture_batch(events).await.unwrap();

        // Give worker time to process
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify 1 batch was sent
        mock.assert_hits(1);
    }

    #[tokio::test]
    async fn test_default_configuration() {
        // Test that default flush_at (100) and flush_interval (500ms) work
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .build()
            .unwrap();

        let client = client(options).await;

        // Send 5 events with default config
        for i in 0..5 {
            let distinct_id = format!("user_{}", i);
            let event = Event::new("test_event", &distinct_id);
            client.capture(event).await.unwrap();
        }

        // Wait for default flush_interval (500ms)
        tokio::time::sleep(Duration::from_millis(700)).await;

        // Verify batch was sent
        mock.assert_hits(1);
    }

    #[tokio::test]
    async fn test_empty_shutdown() {
        // Test that shutdown with no events doesn't cause issues
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        {
            let options = ClientOptionsBuilder::default()
                .api_key("test_key".to_string())
                .api_endpoint(format!("{}/batch/", server.base_url()))
                .build()
                .unwrap();

            let _client = client(options).await;

            // Client drops without capturing any events
        }

        // Give worker time to shut down
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify no requests were made
        mock.assert_hits(0);
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Test that errors from the server are handled gracefully
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(500).body("Internal Server Error");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(3)
            .flush_interval_ms(100)
            .build()
            .unwrap();

        let client = client(options).await;

        // Send events that will trigger a batch with error response
        for i in 0..3 {
            let distinct_id = format!("user_{}", i);
            let event = Event::new("test_event", &distinct_id);
            // capture() should not panic even if server errors
            client.capture(event).await.unwrap();
        }

        // Give worker time to attempt sending (and fail)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify the request was attempted
        mock.assert_hits(1);

        // Send more events to verify worker still works after error
        for i in 3..6 {
            let distinct_id = format!("user_{}", i);
            let event = Event::new("test_event", &distinct_id);
            client.capture(event).await.unwrap();
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify worker continues processing despite previous error
        mock.assert_hits(2);
    }
}
