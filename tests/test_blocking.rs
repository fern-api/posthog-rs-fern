#[cfg(not(feature = "async-client"))]
mod blocking_batching_tests {
    use httpmock::prelude::*;
    use posthog_rs::{client, ClientOptionsBuilder, Event};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_batching_flush_at_threshold() {
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

        let handle = thread::spawn(move || {
            let client = client(options);

            // Send exactly 5 events
            for i in 0..5 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                client.capture(event).unwrap();
            }

            // Give worker time to process and send batch
            thread::sleep(Duration::from_millis(200));
        });

        handle.join().unwrap();

        // Verify exactly 1 batch was sent
        mock.assert_hits(1);
    }

    #[test]
    fn test_batching_flush_interval_timeout() {
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

        let handle = thread::spawn(move || {
            let client = client(options);

            // Send only 3 events (below flush_at)
            for i in 0..3 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                client.capture(event).unwrap();
            }

            // Wait for flush_interval to trigger
            thread::sleep(Duration::from_millis(250));
        });

        handle.join().unwrap();

        // Verify 1 batch was sent
        mock.assert_hits(1);
    }

    #[test]
    fn test_graceful_shutdown_flushes_remaining_events() {
        // Test that Drop implementation flushes remaining events
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            then.status(200).body("");
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(100) // High threshold
            .flush_interval_ms(10000) // Long timeout
            .build()
            .unwrap();

        let handle = thread::spawn(move || {
            {
                let client = client(options);

                // Send 3 events (below both thresholds)
                for i in 0..3 {
                    let distinct_id = format!("user_{}", i);
                    let event = Event::new("test_event", &distinct_id);
                    client.capture(event).unwrap();
                }

                // Client drops here, should flush remaining events
            }

            // Give worker time to flush on shutdown
            thread::sleep(Duration::from_millis(200));
        });

        handle.join().unwrap();

        // Verify the 3 events were flushed
        mock.assert_hits(1);
    }

    #[test]
    fn test_multiple_batches() {
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

        let handle = thread::spawn(move || {
            let client = client(options);

            // Send 15 events (should trigger 3 batches of 5)
            for i in 0..15 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                client.capture(event).unwrap();
            }

            // Give worker time to process all batches
            thread::sleep(Duration::from_millis(500));
        });

        handle.join().unwrap();

        // Verify we received 3 batches
        mock.assert_hits(3);
    }

    #[test]
    fn test_capture_batch_method() {
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

        let handle = thread::spawn(move || {
            let client = client(options);

            // Use capture_batch to send 10 events at once
            let events: Vec<_> = (0..10)
                .map(|i| {
                    let distinct_id = format!("user_{}", i);
                    Event::new("test_event", &distinct_id)
                })
                .collect();

            client.capture_batch(events).unwrap();

            // Give worker time to process
            thread::sleep(Duration::from_millis(200));
        });

        handle.join().unwrap();

        // Verify 1 batch was sent
        mock.assert_hits(1);
    }

    #[test]
    fn test_default_configuration() {
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

        let handle = thread::spawn(move || {
            let client = client(options);

            // Send 5 events with default config
            for i in 0..5 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                client.capture(event).unwrap();
            }

            // Wait for default flush_interval (500ms)
            thread::sleep(Duration::from_millis(700));
        });

        handle.join().unwrap();

        // Verify batch was sent
        mock.assert_hits(1);
    }

    #[test]
    fn test_empty_shutdown() {
        // Test that shutdown with no events doesn't cause issues
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

        let handle = thread::spawn(move || {
            {
                let _client = client(options);

                // Client drops without capturing any events
            }

            // Give worker time to shut down
            thread::sleep(Duration::from_millis(100));
        });

        handle.join().unwrap();

        // Verify no requests were made
        mock.assert_hits(0);
    }

    #[test]
    fn test_error_handling() {
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

        let handle = thread::spawn(move || {
            let client = client(options);

            // Send events that will trigger a batch with error response
            for i in 0..3 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                // capture() should not panic even if server errors
                client.capture(event).unwrap();
            }

            // Give worker time to attempt sending (and fail)
            thread::sleep(Duration::from_millis(200));

            // Send more events to verify worker still works after error
            for i in 3..6 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                client.capture(event).unwrap();
            }

            thread::sleep(Duration::from_millis(200));
        });

        handle.join().unwrap();

        // Verify both batches were attempted despite server errors
        mock.assert_hits(2);
    }
}
