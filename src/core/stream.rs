use crate::types::{StreamEvent, TranslateResponse};
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;

pub type EventStream = Pin<Box<dyn Stream<Item = Result<StreamEvent, crate::error::TranslateError>> + Send>>;

pub struct StreamBuilder {
    tx: mpsc::Sender<Result<StreamEvent, crate::error::TranslateError>>,
    request_id: String,
}

impl StreamBuilder {
    pub fn new(request_id: String) -> (Self, EventStream) {
        let (tx, rx) = mpsc::channel(32);
        let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));
        (Self { tx, request_id }, stream)
    }

    pub async fn send_start(&self) -> Result<(), crate::error::TranslateError> {
        self.tx
            .send(Ok(StreamEvent::Start {
                request_id: self.request_id.clone(),
            }))
            .await
            .map_err(|_| crate::error::TranslateError::Internal("stream closed".into()))
    }

    pub async fn send_delta(&self, text: String) -> Result<(), crate::error::TranslateError> {
        self.tx
            .send(Ok(StreamEvent::Delta { text }))
            .await
            .map_err(|_| crate::error::TranslateError::Internal("stream closed".into()))
    }

    pub async fn send_end(&self, response: TranslateResponse) -> Result<(), crate::error::TranslateError> {
        self.tx
            .send(Ok(StreamEvent::End { response }))
            .await
            .map_err(|_| crate::error::TranslateError::Internal("stream closed".into()))
    }

    pub async fn send_error(&self, err: crate::error::TranslateError) -> Result<(), crate::error::TranslateError> {
        let retryable = err.is_retryable();
        let code = match &err {
            crate::error::TranslateError::ProviderNotFound { .. } => "PROVIDER_NOT_FOUND",
            crate::error::TranslateError::LanguagePairNotSupported { .. } => "LANGUAGE_PAIR_NOT_SUPPORTED",
            crate::error::TranslateError::QuotaExhausted { .. } => "QUOTA_EXHAUSTED",
            crate::error::TranslateError::RateLimited { .. } => "RATE_LIMITED",
            crate::error::TranslateError::AuthenticationFailed { .. } => "AUTHENTICATION_FAILED",
            crate::error::TranslateError::UpstreamError { .. } => "UPSTREAM_ERROR",
            crate::error::TranslateError::NetworkError(_) => "NETWORK_ERROR",
            crate::error::TranslateError::Timeout { .. } => "TIMEOUT",
            crate::error::TranslateError::PreservePatternMismatch { .. } => "PRESERVE_PATTERN_MISMATCH",
            crate::error::TranslateError::InvalidInput(_) => "INVALID_INPUT",
            crate::error::TranslateError::Internal(_) => "INTERNAL",
        };
        self.tx
            .send(Ok(StreamEvent::Error {
                code: code.to_string(),
                message: err.to_string(),
                retryable,
            }))
            .await
            .map_err(|_| crate::error::TranslateError::Internal("stream closed".into()))
    }
}

pub async fn merge_streams(
    streams: Vec<EventStream>,
) -> EventStream {
    let (tx, rx) = mpsc::channel(32);
    let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));

    tokio::spawn(async move {
        let mut streams = streams;
        let mut active = streams.len();

        while active > 0 {
            let mut done = 0;
            for s in &mut streams {
                use futures::StreamExt;
                if let Some(item) = s.next().await {
                    if tx.send(item).await.is_err() {
                        return;
                    }
                } else {
                    done += 1;
                }
            }
            if done == streams.len() {
                active = 0;
            }
            tokio::task::yield_now().await;
        }
    });

    stream
}