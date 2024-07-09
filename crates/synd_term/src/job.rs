use std::{collections::VecDeque, num::NonZero};

use futures_util::{future::BoxFuture, stream::FuturesUnordered, StreamExt as _};

use crate::command::Command;

pub(crate) type JobFuture = BoxFuture<'static, anyhow::Result<Command>>;

pub(crate) struct Jobs {
    futures: FuturesUnordered<JobFuture>,
    background: FuturesUnordered<JobFuture>,
    delay_queue: VecDeque<JobFuture>,
    concurrent_limit: NonZero<usize>,
}

impl Jobs {
    pub fn new(concurrent_limit: NonZero<usize>) -> Self {
        Self {
            futures: FuturesUnordered::new(),
            background: FuturesUnordered::new(),
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

        tracing::trace!(
            "Job delay_queue: {} futures: {}",
            self.delay_queue.len(),
            self.futures.len()
        );
    }

    pub(crate) fn push_background(&mut self, job: JobFuture) {
        self.background.push(job);
    }

    pub(crate) async fn next(&mut self) -> Option<anyhow::Result<Command>> {
        debug_assert!(self.concurrent_limit.get() >= self.futures.len());

        tokio::select! {
           result = self.futures.next() => {
               match result {
                Some(result) => {
                    if let Some(job) = self.delay_queue.pop_front() {
                        self.futures.push(job);
                    }
                    Some(result)
                }
                None => None,
               }
           }
           result = self.background.next() => result,
        }
    }

    #[cfg(feature = "integration")]
    pub(crate) fn is_empty(&self) -> bool {
        self.futures.is_empty()
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
            job.push(future::ready(Ok(Command::Nop)).boxed());
        }

        assert_eq!(job.futures.len(), 2);
        assert_eq!(job.delay_queue.len(), 1);

        let mut count = 0;
        loop {
            if let Some(result) = job.next().await {
                assert!(matches!(result, Ok(Command::Nop)));
                count += 1;
            }
            if count == 3 {
                break;
            }
        }
    }

    #[tokio::test]
    async fn background_job() {
        let mut job = Jobs::new(NonZero::new(2).unwrap());

        job.push(future::ready(Ok(Command::Nop)).boxed());
        job.push(future::ready(Ok(Command::Nop)).boxed());
        job.push_background(future::ready(Ok(Command::Nop)).boxed());

        let mut count = 0;
        loop {
            if let Some(result) = job.next().await {
                assert!(matches!(result, Ok(Command::Nop)));
                count += 1;
            }
            if count == 3 {
                break;
            }
        }
    }
}
