use std::thread;
use std::time::{Duration, Instant};

use log::warn;
use reqwest::{header::CONTENT_TYPE, Client as HttpClient};
use tokio::sync::mpsc;

use crate::{event::InnerEvent, Error, Event};

use super::ClientOptions;

/// A [`Client`] facilitates interactions with the PostHog API over HTTP.
/// Events are queued and batched automatically in a background thread.
///
/// ## Shutdown Behavior
///
/// When the client is dropped, it waits for the background thread to complete,
/// ensuring all remaining events are flushed to PostHog. This provides the same
/// shutdown guarantees as the blocking client.
pub struct Client {
    sender: Option<mpsc::Sender<Event>>,
    #[allow(dead_code)] // Used in Drop
    worker_handle: Option<thread::JoinHandle<()>>,
}

/// This function constructs a new client using the options provided.
pub async fn client<C: Into<ClientOptions>>(options: C) -> Client {
    let options = options.into();
    let http_client = HttpClient::builder()
        .timeout(Duration::from_secs(options.request_timeout_seconds))
        .build()
        .unwrap(); // Unwrap here is as safe as `HttpClient::new`

    // Create bounded channel for event queueing
    let (sender, receiver) = mpsc::channel(options.max_queue_size);

    // Spawn background worker thread with embedded tokio runtime
    // Using a thread (instead of tokio task) allows us to join() in Drop,
    // guaranteeing all events are flushed on shutdown
    let worker_handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for worker thread");

        rt.block_on(async move {
            worker_loop(receiver, options, http_client).await;
        });
    });

    Client {
        sender: Some(sender),
        worker_handle: Some(worker_handle),
    }
}

/// Background worker loop that collects and batches events
async fn worker_loop(
    mut receiver: mpsc::Receiver<Event>,
    options: ClientOptions,
    client: HttpClient,
) {
    loop {
        let mut batch = Vec::with_capacity(options.flush_at);
        let deadline = Instant::now() + Duration::from_millis(options.flush_interval_ms);

        // Collect events until flush_at or timeout
        while batch.len() < options.flush_at {
            let timeout = deadline.saturating_duration_since(Instant::now());

            match tokio::time::timeout(timeout, receiver.recv()).await {
                Ok(Some(event)) => batch.push(event),
                Ok(None) => {
                    // Channel closed, flush remaining events and exit
                    if !batch.is_empty() {
                        if let Err(e) = send_batch(&client, &options, batch).await {
                            warn!("Failed to send final batch on shutdown: {}", e);
                        }
                    }
                    return;
                }
                Err(_) => break, // Timeout, flush now
            }
        }

        // Send batch if we have events
        if !batch.is_empty() {
            let batch_size = batch.len();
            if let Err(e) = send_batch(&client, &options, batch).await {
                warn!("Failed to send batch of {} events: {}", batch_size, e);
            }
        }
    }
}

/// Send a batch of events to PostHog
async fn send_batch(
    client: &HttpClient,
    options: &ClientOptions,
    events: Vec<Event>,
) -> Result<(), Error> {
    let inner_events: Vec<_> = events
        .into_iter()
        .map(|event| InnerEvent::new(event, options.api_key.clone()))
        .collect();

    let payload =
        serde_json::to_string(&inner_events).map_err(|e| Error::Serialization(e.to_string()))?;

    client
        .post(&options.api_endpoint)
        .header(CONTENT_TYPE, "application/json")
        .body(payload)
        .send()
        .await
        .map_err(|e| Error::Connection(e.to_string()))?;

    Ok(())
}

impl Client {
    /// Capture the provided event, queuing it for batch sending.
    ///
    /// This method returns immediately after queueing the event - it does **not**
    /// wait for the event to be sent to PostHog. The actual HTTP request happens
    /// asynchronously in a background thread.
    ///
    /// Events are automatically batched and sent when either:
    /// - The batch reaches `flush_at` events (default: 100)
    /// - `flush_interval` milliseconds have elapsed (default: 500ms)
    ///
    /// # Error Handling
    ///
    /// This method only returns errors for queueing failures (e.g., client already
    /// shut down). Network errors and HTTP failures that occur during the actual
    /// send operation are logged via the `log` crate but are not returned to the
    /// caller. Initialize a logger (e.g., `env_logger::init()`) to see these warnings.
    ///
    /// If you need guaranteed delivery confirmation, consider implementing a
    /// separate verification mechanism.
    pub async fn capture(&self, event: Event) -> Result<(), Error> {
        self.sender
            .as_ref()
            .ok_or_else(|| Error::Connection("Client has been shut down".to_string()))?
            .send(event)
            .await
            .map_err(|_| Error::Connection("Event queue closed".to_string()))
    }

    /// Capture a collection of events, queuing each for batch sending.
    /// This method does not block - events are sent asynchronously by a background task.
    pub async fn capture_batch(&self, events: Vec<Event>) -> Result<(), Error> {
        for event in events {
            self.capture(event).await?;
        }
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Drop sender first to signal worker to shutdown
        drop(self.sender.take());
        // Wait for worker thread to finish (will flush remaining events)
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}
