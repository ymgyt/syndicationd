use crate::{
    crawl::job::{
        ClaimCrawlJobCommand, ClaimCrawlJobOutcome, EnqueueCrawlJobCommand, EnqueueCrawlJobOutcome,
        FinishCrawlJobCommand, FinishCrawlJobOutcome,
    },
    db::CrawlJobQueueTx,
    error::RegistryDbResult,
    event::{CrawlJobEnqueuedEvent, CrawlJobStartedEvent, Event, JournalTx, RecordedEvents},
};

/// Transactional service for durable crawl-job queue operations.
pub struct CrawlJobQueue<'a, Tx> {
    tx: &'a mut Tx,
    recorded: &'a mut RecordedEvents,
}

impl<'a, Tx> CrawlJobQueue<'a, Tx> {
    pub fn new(tx: &'a mut Tx, recorded: &'a mut RecordedEvents) -> Self {
        Self { tx, recorded }
    }
}

impl<Tx> CrawlJobQueue<'_, Tx>
where
    Tx: CrawlJobQueueTx + JournalTx + Send,
{
    pub async fn enqueue(
        &mut self,
        command: EnqueueCrawlJobCommand,
    ) -> RegistryDbResult<EnqueueCrawlJobOutcome> {
        let outcome = self.tx.enqueue_job(command).await?;
        if let EnqueueCrawlJobOutcome::Enqueued(job) = &outcome {
            self.record_event(CrawlJobEnqueuedEvent::from(job.clone()))
                .await?;
        }
        Ok(outcome)
    }

    pub async fn claim(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> RegistryDbResult<ClaimCrawlJobOutcome> {
        let outcome = self.tx.claim_job(command).await?;
        if let ClaimCrawlJobOutcome::Claimed(job) = &outcome {
            self.record_event(CrawlJobStartedEvent::from(job.clone()))
                .await?;
        }
        Ok(outcome)
    }

    pub async fn finish(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> RegistryDbResult<FinishCrawlJobOutcome> {
        self.tx.finish_job(command).await
    }

    async fn record_event<E>(&mut self, event: E) -> RegistryDbResult<()>
    where
        E: Into<Event>,
    {
        let event = event.into();
        let kind = event.kind();
        self.tx.append_event(event).await?;
        self.recorded.push(kind);
        Ok(())
    }
}
