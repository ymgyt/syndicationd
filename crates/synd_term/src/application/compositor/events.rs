use super::Compositor;
use crate::{
    application::effects::ComponentEffects,
    event::{Event, EventKind},
    operation::Operations,
};
use tracing::debug;

impl Compositor {
    pub(in crate::application) fn apply_event(&mut self, event: &Event) -> Operations {
        let replies = self.notify(std::slice::from_ref(event));
        let operations = self.run(replies);
        self.sync_keymap();
        operations
    }

    /// Deliver facts (including the facts they produce) before executing follow-up commands.
    pub(super) fn notify(&mut self, events: &[Event]) -> Vec<ComponentEffects> {
        let mut replies = Vec::new();
        for event in events {
            if self
                .in_flight
                .observe(event)
                .is_some_and(|kind| self.components.gh.is_stale(kind))
            {
                continue;
            }
            let subscribers = self
                .subscribers
                .get(&EventKind::from(event))
                .cloned()
                .unwrap_or_default();
            debug!(%event, ?subscribers, "Event");
            for id in subscribers {
                let mut effects = self.components.get_mut(id).apply_event(event);
                replies.extend(self.notify(&std::mem::take(&mut effects.events)));
                replies.push(effects.with_sender(id));
            }
        }
        replies
    }
}
