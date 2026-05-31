use std::{collections::VecDeque, num::NonZero};

use futures_util::{StreamExt as _, future::BoxFuture, stream::FuturesUnordered};
use tracing::trace;

use crate::event::Event;

pub(crate) type JobFuture = BoxFuture<'static, anyhow::Result<Event>>;

pub(crate) struct Jobs {
    futures: FuturesUnordered<JobFuture>,
    delay_queue: VecDeque<JobFuture>,
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

    pub(crate) fn push(&mut self, job: JobFuture) {
        self.delay_queue.push_back(job);

        while self.concurrent_limit.get() > self.futures.len() {
            let Some(job) = self.delay_queue.pop_front() else {
                break;
            };

            self.futures.push(job);
        }

        trace!(
            "Job delay_queue: {} futures: {}",
            self.delay_queue.len(),
            self.futures.len()
        );
    }

    pub(crate) async fn next(&mut self) -> Option<anyhow::Result<Event>> {
        debug_assert!(self.concurrent_limit.get() >= self.futures.len());

        match self.futures.next().await {
            Some(result) => {
                if let Some(job) = self.delay_queue.pop_front() {
                    self.futures.push(job);
                }
                Some(result)
            }
            None => None,
        }
    }

    #[cfg(feature = "integration")]
    pub(crate) fn is_empty(&self) -> bool {
        self.futures.is_empty() && self.delay_queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt as _;

    use super::*;
    use std::future;

    #[tokio::test]
    async fn respect_concurrent_limit() {
        let mut job = Jobs::new(NonZero::new(2).unwrap());

        for _ in 0..3 {
            job.push(future::ready(Ok(Event::Nop)).boxed());
        }

        assert_eq!(job.futures.len(), 2);
        assert_eq!(job.delay_queue.len(), 1);

        let mut count = 0;
        loop {
            if let Some(result) = job.next().await {
                assert!(matches!(result, Ok(Event::Nop)));
                count += 1;
            }
            if count == 3 {
                break;
            }
        }
    }
}
