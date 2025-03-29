//! Context propagation for the logging system
//!
//! This module provides utilities for propagating context across async boundaries,
//! including correlation IDs, span context, and other metadata.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tracing::{Instrument, Span};
use tracing_futures::Instrumented;

use crate::Logger;

/// A future that propagates context
pub struct ContextPropagatedFuture<F> {
    inner: Instrumented<F>,
    correlation_id: Option<String>,
}

impl<F: Future> Future for ContextPropagatedFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        
        // Save the current correlation ID
        let old_correlation_id = Logger::correlation_id();
        
        // Set the correlation ID from this future
        if let Some(id) = &this.correlation_id {
            Logger::set_correlation_id(id.clone());
        }
        
        // Poll the inner future
        let result = unsafe { Pin::new_unchecked(&mut this.inner) }.poll(cx);
        
        // Restore the original correlation ID
        match old_correlation_id {
            Some(id) => Logger::set_correlation_id(id),
            None => CORRELATION_ID.with(|cell| *cell.borrow_mut() = None),
        }
        
        result
    }
}

/// Extension trait for futures to propagate context
pub trait ContextPropagation: Sized {
    /// Propagate the current context to the future
    fn propagate_context(self) -> ContextPropagatedFuture<Self>;
    
    /// Propagate the current context and instrument with the given span
    fn propagate_context_with_span(self, span: Span) -> ContextPropagatedFuture<Instrumented<Self>>;
}

impl<F: Future> ContextPropagation for F {
    fn propagate_context(self) -> ContextPropagatedFuture<Self> {
        ContextPropagatedFuture {
            inner: self.instrument(Span::current()),
            correlation_id: Logger::correlation_id(),
        }
    }
    
    fn propagate_context_with_span(self, span: Span) -> ContextPropagatedFuture<Instrumented<Self>> {
        ContextPropagatedFuture {
            inner: self.instrument(span),
            correlation_id: Logger::correlation_id(),
        }
    }
}

/// Thread-local storage for correlation ID
thread_local! {
    pub(crate) static CORRELATION_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// A decorator for async functions that propagates context
#[macro_export]
macro_rules! with_context {
    ($func:expr) => {
        $func.propagate_context()
    };
    ($func:expr, $span:expr) => {
        $func.propagate_context_with_span($span)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    
    #[test]
    fn test_context_propagation() {
        // Set a correlation ID
        let test_id = "test-correlation-id";
        Logger::set_correlation_id(test_id);
        
        // Create an async function that checks the correlation ID
        async fn check_correlation_id() -> String {
            Logger::correlation_id().unwrap_or_default()
        }
        
        // Run with context propagation
        let result = block_on(check_correlation_id().propagate_context());
        assert_eq!(result, test_id);
        
        // Run with a different span
        let span = tracing::info_span!("test_span");
        let result = block_on(check_correlation_id().propagate_context_with_span(span));
        assert_eq!(result, test_id);
    }
}