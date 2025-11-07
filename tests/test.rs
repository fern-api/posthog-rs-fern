#[cfg(feature = "e2e-test")]
mod e2e_tests {
    use dotenv::dotenv;
    use posthog_rs::{client, ClientOptionsBuilder, Event};
    use std::collections::HashMap;

    fn get_api_key() -> String {
        dotenv().ok(); // Load the .env file
        std::env::var("POSTHOG_RS_E2E_TEST_API_KEY")
            .expect("POSTHOG_RS_E2E_TEST_API_KEY must be set in .env")
    }

    #[cfg(not(feature = "async-client"))]
    mod blocking_tests {
        use super::*;
        use std::thread;
        use std::time::Duration;

        #[test]
        fn test_blocking_single_event() {
            // see https://us.posthog.com/project/115809/ for the e2e project
            println!("Testing blocking client with single event");

            let api_key = get_api_key();
            let client = client(api_key.as_str());

            let mut child_map = HashMap::new();
            child_map.insert("child_key1", "child_value1");

            let mut event = Event::new("e2e test event", "1234");
            event.insert_prop("key1", "value1").unwrap();
            event.insert_prop("key2", vec!["a", "b"]).unwrap();
            event.insert_prop("key3", child_map).unwrap();

            client.capture(event).unwrap();

            // Give time for batching to flush
            thread::sleep(Duration::from_secs(2));

            println!("✅ Blocking single event test completed");
        }

        #[test]
        fn test_blocking_batching_default_config() {
            println!("Testing blocking client with default batching config");

            let api_key = get_api_key();
            let client = client(api_key.as_str());

            // Send 10 events with default batching
            for i in 0..10 {
                let distinct_id = format!("e2e_user_{}", i);
                let mut event = Event::new("e2e batching test", &distinct_id);
                event
                    .insert_prop("test_run", "default_config")
                    .unwrap();
                event.insert_prop("event_number", i).unwrap();

                client.capture(event).unwrap();
            }

            // Give time for batching to flush (default is 500ms)
            thread::sleep(Duration::from_secs(2));

            println!("✅ Blocking batching (default config) test completed");
        }

        #[test]
        fn test_blocking_batching_custom_config() {
            println!("Testing blocking client with custom batching config");

            let api_key = get_api_key();
            let options = ClientOptionsBuilder::default()
                .api_key(api_key)
                .flush_at(5)
                .flush_interval_ms(1000)
                .build()
                .unwrap();

            let client = client(options);

            // Send 15 events (should trigger 3 batches of 5)
            for i in 0..15 {
                let distinct_id = format!("e2e_user_{}", i);
                let mut event = Event::new("e2e batching test", &distinct_id);
                event.insert_prop("test_run", "custom_config").unwrap();
                event.insert_prop("event_number", i).unwrap();

                client.capture(event).unwrap();
            }

            // Give time for batching to complete
            thread::sleep(Duration::from_secs(3));

            println!("✅ Blocking batching (custom config) test completed");
        }
    }

    #[cfg(feature = "async-client")]
    mod async_tests {
        use super::*;
        use tokio::time::{sleep, Duration};

        #[tokio::test]
        async fn test_async_single_event() {
            // see https://us.posthog.com/project/115809/ for the e2e project
            println!("Testing async client with single event");

            let api_key = get_api_key();
            let client = client(api_key.as_str()).await;

            let mut child_map = HashMap::new();
            child_map.insert("child_key1", "child_value1");

            let mut event = Event::new("e2e test event async", "1234");
            event.insert_prop("key1", "value1").unwrap();
            event.insert_prop("key2", vec!["a", "b"]).unwrap();
            event.insert_prop("key3", child_map).unwrap();

            client.capture(event).await.unwrap();

            // Give time for batching to flush
            sleep(Duration::from_secs(2)).await;

            println!("✅ Async single event test completed");
        }

        #[tokio::test]
        async fn test_async_batching_default_config() {
            println!("Testing async client with default batching config");

            let api_key = get_api_key();
            let client = client(api_key.as_str()).await;

            // Send 10 events with default batching
            for i in 0..10 {
                let distinct_id = format!("e2e_user_async_{}", i);
                let mut event = Event::new("e2e batching test async", &distinct_id);
                event
                    .insert_prop("test_run", "default_config")
                    .unwrap();
                event.insert_prop("event_number", i).unwrap();

                client.capture(event).await.unwrap();
            }

            // Give time for batching to flush (default is 500ms)
            sleep(Duration::from_secs(2)).await;

            println!("✅ Async batching (default config) test completed");
        }

        #[tokio::test]
        async fn test_async_batching_custom_config() {
            println!("Testing async client with custom batching config");

            let api_key = get_api_key();
            let options = ClientOptionsBuilder::default()
                .api_key(api_key)
                .flush_at(5)
                .flush_interval_ms(1000)
                .build()
                .unwrap();

            let client = client(options).await;

            // Send 15 events (should trigger 3 batches of 5)
            for i in 0..15 {
                let distinct_id = format!("e2e_user_async_{}", i);
                let mut event = Event::new("e2e batching test async", &distinct_id);
                event.insert_prop("test_run", "custom_config").unwrap();
                event.insert_prop("event_number", i).unwrap();

                client.capture(event).await.unwrap();
            }

            // Give time for batching to complete
            sleep(Duration::from_secs(3)).await;

            println!("✅ Async batching (custom config) test completed");
        }
    }
}
