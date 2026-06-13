use crate::{
    crawl::job::{
        ClaimCrawlJobCommand, ClaimCrawlJobOutcome, EnqueueCrawlJobCommand, EnqueueCrawlJobOutcome,
        FinishCrawlJobCommand, FinishCrawlJobOutcome,
    },
    db::CrawlJobQueueTx,
    error::RegistryDbResult,
    event::{CrawlJobEnqueuedEvent, CrawlJobStartedEvent, Event},
};

/// Transactional service for durable crawl-job queue operations.
pub struct CrawlJobQueue<'a, Tx> {
    tx: &'a mut Tx,
}

impl<'a, Tx> CrawlJobQueue<'a, Tx> {
    pub fn new(tx: &'a mut Tx) -> Self {
        Self { tx }
    }
}

impl<Tx> CrawlJobQueue<'_, Tx>
where
    Tx: CrawlJobQueueTx + Send,
{
    pub async fn enqueue(
        &mut self,
        command: EnqueueCrawlJobCommand,
    ) -> RegistryDbResult<(EnqueueCrawlJobOutcome, Vec<Event>)> {
        let outcome = self.tx.enqueue_job(command).await?;
        let events = match &outcome {
            EnqueueCrawlJobOutcome::Enqueued(job) => {
                vec![CrawlJobEnqueuedEvent::from(job.clone()).into()]
            }
            EnqueueCrawlJobOutcome::AlreadyActive => Vec::new(),
        };
        Ok((outcome, events))
    }

    pub async fn claim(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> RegistryDbResult<(ClaimCrawlJobOutcome, Vec<Event>)> {
        let outcome = self.tx.claim_job(command).await?;
        let events = match &outcome {
            ClaimCrawlJobOutcome::Claimed(job) => {
                vec![CrawlJobStartedEvent::from(job.clone()).into()]
            }
            ClaimCrawlJobOutcome::NoClaimableJob => Vec::new(),
        };
        Ok((outcome, events))
    }

    pub async fn finish(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> RegistryDbResult<FinishCrawlJobOutcome> {
        self.tx.finish_job(command).await
    }
}
