use std::time::{Duration, SystemTime};
use tokio::task::JoinHandle;
use reqwest::Response;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::oneshot::{Sender, channel};
use thiserror::Error;
use log::info;
use std::ops::Sub;
use async_recursion::async_recursion;

#[derive(Clone)]
pub struct RequestManager {
    client: reqwest::Client,
    queue: tokio::sync::mpsc::Sender<(reqwest::Request, Sender<Result<Response, RequestError>>)>,
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Request returned an invalid status code: {0}")]
    StatusCodeError(u16),
    #[error("An error occurred in the upstream request: {0}")]
    ReqwestError(reqwest::Error),
    #[error("Failed to acquire request lock: {0}")]
    SemaphoreError(tokio::sync::AcquireError),
    #[error("Failed to build request: {0}")]
    RequestBuildError(reqwest::Error),
    #[error("Failed to submit request: Queue is full")]
    BufferError(),
    #[error("An error occurred when awaiting the result response: {0}")]
    RecvError(tokio::sync::oneshot::error::RecvError),
    #[error("An error occurred when deserializing received json: {0}")]
    JsonError(reqwest::Error),
}

impl RequestManager {
    pub fn new(client: reqwest::Client, concurrent_requests: u8, max_requests: u8, ratelimit_window: Duration) -> (Self, JoinHandle<()>) {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
        let manager = Self {
            client: client.clone(),
            queue: tx,
        };
        (manager, tokio::spawn(async move {
            let mut last_reset = SystemTime::UNIX_EPOCH;
            let mut current_window_requests = 0u8;
            let client = client.clone();
            let semaphore = Arc::new(Semaphore::new(concurrent_requests as usize));
            let max_requests = max_requests;
            let ratelimit_window = ratelimit_window.clone();
            loop {
                if let Some((request, response_channel)) = rx.recv().await {
                    if last_reset.elapsed().map(|duration| duration.gt(&ratelimit_window)).unwrap_or(false) {
                        current_window_requests = 1;
                        last_reset = SystemTime::now();
                    } else {
                        if current_window_requests >= max_requests {
                            let passed = last_reset.elapsed().unwrap_or(Duration::from_secs(0));
                            let wait = ratelimit_window.sub(passed);
                            tokio::time::sleep(wait).await;
                            current_window_requests = 1;
                            last_reset = SystemTime::now();
                        } else {
                            current_window_requests += 1;
                        }
                    }

                    match semaphore.clone().acquire_owned().await {
                        Ok(permit) => {
                            let client = client.clone();
                            tokio::spawn(async move {
                                let result = RequestManager::execute_request(client, request).await;
                                response_channel.send(result).ok();
                                drop(permit)
                            });
                        }
                        Err(err) => {
                            response_channel.send(Err(RequestError::SemaphoreError(err))).ok();
                        }
                    }
                }
            }
        }))
    }

    #[async_recursion]
    async fn execute_request(client: reqwest::Client, request: reqwest::Request) -> Result<Response, RequestError> {
        let optional_clone = request.try_clone();
        let response_result = client.execute(request).await;
        match response_result {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response)
                } else {
                    let status_code = response.status().as_u16();
                    if status_code == 429 {
                        info!("429 Status Headers");
                        response.headers().iter().for_each(|(key, value)| {
                            info!("{:?}: {:?}", key, value);
                        });
                        tokio::time::sleep(Duration::from_millis(1500)).await;
                        if let Some(request) = optional_clone {
                            let result = Self::execute_request(client, request).await;
                            result
                        } else {
                            Err(RequestError::StatusCodeError(status_code))
                        }
                    } else {
                        Err(RequestError::StatusCodeError(status_code))
                    }
                }
            }
            Err(err) => {
                Err(RequestError::ReqwestError(err))
            }
        }
    }

    pub async fn queue_request<F: FnOnce(&reqwest::Client) -> reqwest::Result<reqwest::Request>>(&self, caller: F) -> Result<Response, RequestError> {
        let (tx, rx) = channel();
        let request = caller(&self.client)
            .map_err(|err| RequestError::RequestBuildError(err))?;
        self.queue.send((request, tx)).await
            .map_err(|_| RequestError::BufferError())?;
        rx.await.map_err(|err| RequestError::RecvError(err))?
    }
}