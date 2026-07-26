use std::{
    collections::VecDeque,
    num::NonZero,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{Stream, StreamExt as _, future::BoxFuture, stream::FuturesUnordered};
use tracing::trace;

/// Polls a bounded set of jobs without terminating while temporarily empty.
pub(crate) struct Jobs {
    futures: FuturesUnordered<BoxFuture<'static, ()>>,
    delay_queue: VecDeque<BoxFuture<'static, ()>>,
    concurrent_limit: NonZero<usize>,
}

impl Jobs {
    pub fn new(concurrent_limit: NonZero<usize>) -> Self {
        Self {
            futures: FuturesUnordered::new(),
            delay_queue: VecDeque::new(),
            concurrent_limit,
        }
    }

    pub(crate) fn push(&mut self, job: BoxFuture<'static, ()>) {
        self.delay_queue.push_back(job);
        self.admit_delayed();

        trace!(
            "Job delay_queue: {} futures: {}",
            self.delay_queue.len(),
            self.futures.len()
        );
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.futures.is_empty() && self.delay_queue.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.futures.clear();
        self.delay_queue.clear();
    }

    fn admit_delayed(&mut self) {
        while self.concurrent_limit.get() > self.futures.len() {
            let Some(job) = self.delay_queue.pop_front() else {
                break;
            };

            self.futures.push(job);
        }
    }
}

impl Stream for Jobs {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        debug_assert!(self.concurrent_limit.get() >= self.futures.len());

        match self.futures.poll_next_unpin(cx) {
            Poll::Ready(Some(())) => {
                self.admit_delayed();
                Poll::Ready(Some(()))
            }
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{FutureExt as _, StreamExt as _};

    use super::*;
    use std::future;

    #[tokio::test]
    async fn respect_concurrent_limit() {
        let mut job = Jobs::new(NonZero::new(2).unwrap());

        for _ in 0..3 {
            job.push(future::ready(()).boxed());
        }

        assert_eq!(job.futures.len(), 2);
        assert_eq!(job.delay_queue.len(), 1);

        let mut count = 0;
        loop {
            if job.next().await.is_some() {
                count += 1;
            }
            if count == 3 {
                break;
            }
        }

        assert!(job.next().now_or_never().is_none());
    }
}
