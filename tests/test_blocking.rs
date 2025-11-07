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
                // Drop blocks until worker thread finishes
            }
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

    #[test]
    fn test_backpressure() {
        // Test that capture blocks when queue is full (backpressure)
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/batch/");
            // Slow response to allow queue to fill up
            then.status(200).body("").delay(Duration::from_millis(100));
        });

        let options = ClientOptionsBuilder::default()
            .api_key("test_key".to_string())
            .api_endpoint(format!("{}/batch/", server.base_url()))
            .flush_at(5)
            .max_queue_size(10) // Small queue to test backpressure
            .flush_interval_ms(10000)
            .build()
            .unwrap();

        let handle = thread::spawn(move || {
            let client = client(options);

            // Send 20 events quickly - this will fill the queue (10 capacity)
            // and trigger backpressure
            for i in 0..20 {
                let distinct_id = format!("user_{}", i);
                let event = Event::new("test_event", &distinct_id);
                // This should block when queue is full, then continue when space is available
                client.capture(event).unwrap();
            }

            // Drop client to flush remaining events
            drop(client);
        });

        handle.join().unwrap();

        // Client dropped in thread, worker thread joined before thread exits
        // Verify all 20 events were sent (4 batches of 5)
        mock.assert_hits(4);
    }

    #[test]
    fn test_concurrent_captures() {
        // Test that multiple concurrent threads can safely capture events
        use std::sync::Arc;

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

        let client = Arc::new(client(options));

        // Spawn 10 concurrent threads, each sending 10 events (100 total)
        let mut handles = vec![];
        for task_id in 0..10 {
            let client = Arc::clone(&client);
            let handle = thread::spawn(move || {
                for i in 0..10 {
                    let distinct_id = format!("thread_{}_event_{}", task_id, i);
                    let event = Event::new("concurrent_test", &distinct_id);
                    client.capture(event).unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Drop client to flush remaining events
        // Drop blocks until worker thread finishes (via handle.join())
        drop(client);

        // Verify all 100 events were sent (10 batches of 10)
        mock.assert_hits(10);
    }
}
