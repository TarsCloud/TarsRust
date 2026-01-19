//! # Filter Module
//!
//! Request filter/middleware support for client and server.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::Result;
use crate::protocol::{RequestPacket, ResponsePacket};
use crate::util::Context;

/// Message for filter chain
pub struct Message {
    /// Request packet
    pub req: RequestPacket,
    /// Response packet
    pub resp: Option<ResponsePacket>,
    /// Begin time (unix ms)
    pub begin_time: i64,
    /// End time (unix ms)
    pub end_time: i64,
    /// Status code
    pub status: i32,
    /// Hash code for hash-based routing
    pub hash_code: u32,
    /// Hash type
    pub hash_type: crate::selector::HashType,
    /// Is hash-based call
    pub is_hash: bool,
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    pub fn new() -> Self {
        Self {
            req: RequestPacket::new(),
            resp: None,
            begin_time: chrono::Utc::now().timestamp_millis(),
            end_time: 0,
            status: 0,
            hash_code: 0,
            hash_type: crate::selector::HashType::ModHash,
            is_hash: false,
        }
    }

    pub fn with_request(req: RequestPacket) -> Self {
        Self {
            req,
            ..Self::new()
        }
    }

    pub fn finish(&mut self) {
        self.end_time = chrono::Utc::now().timestamp_millis();
    }

    pub fn elapsed_ms(&self) -> i64 {
        if self.end_time > 0 {
            self.end_time - self.begin_time
        } else {
            chrono::Utc::now().timestamp_millis() - self.begin_time
        }
    }
}

impl crate::selector::Message for Message {
    fn hash_code(&self) -> u32 {
        self.hash_code
    }

    fn hash_type(&self) -> crate::selector::HashType {
        self.hash_type
    }

    fn is_hash(&self) -> bool {
        self.is_hash
    }
}

/// Client invoke function type
pub type InvokeFn = Arc<
    dyn Fn(Context, Message, Duration) -> Pin<Box<dyn Future<Output = Result<Message>> + Send>>
        + Send
        + Sync,
>;

/// Client filter function
pub type ClientFilter = Arc<
    dyn Fn(
            Context,
            Message,
            InvokeFn,
            Duration,
        ) -> Pin<Box<dyn Future<Output = Result<Message>> + Send>>
        + Send
        + Sync,
>;

/// Client filter middleware
pub type ClientFilterMiddleware = Arc<dyn Fn(ClientFilter) -> ClientFilter + Send + Sync>;

/// Server dispatch function type
pub type DispatchFn = Arc<
    dyn Fn(Context, Arc<dyn std::any::Any + Send + Sync>, RequestPacket, bool) -> Pin<Box<dyn Future<Output = Result<ResponsePacket>> + Send>>
        + Send
        + Sync,
>;

/// Server filter function
pub type ServerFilter = Arc<
    dyn Fn(
            Context,
            DispatchFn,
            Arc<dyn std::any::Any + Send + Sync>,
            RequestPacket,
            bool,
        ) -> Pin<Box<dyn Future<Output = Result<ResponsePacket>> + Send>>
        + Send
        + Sync,
>;

/// Server filter middleware
pub type ServerFilterMiddleware = Arc<dyn Fn(ServerFilter) -> ServerFilter + Send + Sync>;

/// Filter chain management
#[derive(Default)]
pub struct Filters {
    /// Client filter (legacy)
    pub client_filter: Option<ClientFilter>,
    /// Pre-invoke client filters
    pub pre_client_filters: Vec<ClientFilter>,
    /// Post-invoke client filters
    pub post_client_filters: Vec<ClientFilter>,
    /// Client filter middlewares
    pub client_middlewares: Vec<ClientFilterMiddleware>,
    /// Server filter (legacy)
    pub server_filter: Option<ServerFilter>,
    /// Pre-invoke server filters
    pub pre_server_filters: Vec<ServerFilter>,
    /// Post-invoke server filters
    pub post_server_filters: Vec<ServerFilter>,
    /// Server filter middlewares
    pub server_middlewares: Vec<ServerFilterMiddleware>,
}

impl Filters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register client filter (legacy)
    pub fn register_client_filter(&mut self, filter: ClientFilter) {
        self.client_filter = Some(filter);
    }

    /// Register pre-invoke client filter
    pub fn register_pre_client_filter(&mut self, filter: ClientFilter) {
        self.pre_client_filters.push(filter);
    }

    /// Register post-invoke client filter
    pub fn register_post_client_filter(&mut self, filter: ClientFilter) {
        self.post_client_filters.push(filter);
    }

    /// Use client filter middleware (recommended)
    pub fn use_client_middleware(&mut self, middleware: ClientFilterMiddleware) {
        self.client_middlewares.push(middleware);
    }

    /// Register server filter (legacy)
    pub fn register_server_filter(&mut self, filter: ServerFilter) {
        self.server_filter = Some(filter);
    }

    /// Register pre-invoke server filter
    pub fn register_pre_server_filter(&mut self, filter: ServerFilter) {
        self.pre_server_filters.push(filter);
    }

    /// Register post-invoke server filter
    pub fn register_post_server_filter(&mut self, filter: ServerFilter) {
        self.post_server_filters.push(filter);
    }

    /// Use server filter middleware (recommended)
    pub fn use_server_middleware(&mut self, middleware: ServerFilterMiddleware) {
        self.server_middlewares.push(middleware);
    }

    /// Build middleware chain for client
    pub fn build_client_filter(&self, invoke: InvokeFn) -> ClientFilter {
        // Create base filter that calls invoke
        let base: ClientFilter = Arc::new(move |ctx, msg, _invoke, timeout| {
            let invoke = Arc::clone(&invoke);
            Box::pin(async move { invoke(ctx, msg, timeout).await })
        });

        // Apply middlewares from last to first
        let mut current = base;
        for middleware in self.client_middlewares.iter().rev() {
            current = middleware(current);
        }

        current
    }

    /// Build middleware chain for server
    pub fn build_server_filter(&self, dispatch: DispatchFn) -> ServerFilter {
        // Create base filter that calls dispatch
        let base: ServerFilter = Arc::new(move |ctx, _dispatch, imp, req, with_ctx| {
            let dispatch = Arc::clone(&dispatch);
            Box::pin(async move { dispatch(ctx, imp, req, with_ctx).await })
        });

        // Apply middlewares from last to first
        let mut current = base;
        for middleware in self.server_middlewares.iter().rev() {
            current = middleware(current);
        }

        current
    }
}

/// Create a logging middleware for client
pub fn logging_middleware() -> ClientFilterMiddleware {
    Arc::new(|next| {
        Arc::new(move |ctx, msg, invoke, timeout| {
            let next = Arc::clone(&next);
            Box::pin(async move {
                let servant = msg.req.s_servant_name.clone();
                let func = msg.req.s_func_name.clone();
                tracing::debug!("Request: {}.{}", servant, func);

                let result = next(ctx, msg, invoke, timeout).await;

                match &result {
                    Ok(msg) => {
                        tracing::debug!(
                            "Response: {}.{} cost={}ms",
                            servant,
                            func,
                            msg.elapsed_ms()
                        );
                    }
                    Err(e) => {
                        tracing::error!("Error: {}.{} err={}", servant, func, e);
                    }
                }

                result
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message() {
        let mut msg = Message::new();
        assert!(msg.begin_time > 0);
        assert_eq!(msg.end_time, 0);

        msg.finish();
        assert!(msg.end_time >= msg.begin_time);
    }

    #[test]
    fn test_filters() {
        let mut filters = Filters::new();
        filters.use_client_middleware(logging_middleware());
        assert_eq!(filters.client_middlewares.len(), 1);
    }
}
