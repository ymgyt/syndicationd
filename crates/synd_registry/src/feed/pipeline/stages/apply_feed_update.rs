use crate::{
    db::FeedDb,
    entry::Change,
    event::{EntryChangedEvent, EntryDiscoveredEvent, Event, ProcessorResult},
    feed::FeedUpdate,
    feed::pipeline::{Ctx, Stage},
};

pub(in crate::feed::pipeline) struct ApplyFeedUpdate;

impl Stage for ApplyFeedUpdate {
    type Input = FeedUpdate;
    type Output = Vec<Event>;

    async fn run<Tx: FeedDb + Send>(
        &self,
        update: FeedUpdate,
        ctx: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Vec<Event>> {
        ctx.tx.apply_feed_update(&update).await?;
        let source = update.source();
        let mut events = Vec::new();
        let mut discovered = 0;
        let mut changed = 0;
        for change in update.entry_changes() {
            let id = change.entry().entry().id().clone();
            events.push(match change {
                Change::Discovered(_) => {
                    discovered += 1;
                    EntryDiscoveredEvent::new(source.feed_url.clone(), id).into()
                }
                Change::Changed(_) => {
                    changed += 1;
                    EntryChangedEvent::new(source.feed_url.clone(), id).into()
                }
            });
        }
        tracing::debug!(feed_url = %source.feed_url, job_id = %source.crawl_job_id, discovered, changed, "feed state projected");
        Ok(events)
    }
}
