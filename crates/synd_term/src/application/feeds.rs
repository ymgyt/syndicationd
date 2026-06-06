use super::Application;

impl<Term, Sess> Application<Term, Sess> {
    pub(super) fn next_entries_first(&self, loaded_after_response: usize) -> i64 {
        let remaining = self
            .config
            .entries_limit
            .saturating_sub(loaded_after_response);
        let page_size = usize::try_from(self.config.entries_per_pagination).unwrap_or(0);

        remaining.min(page_size).try_into().unwrap_or(i64::MAX)
    }
}
