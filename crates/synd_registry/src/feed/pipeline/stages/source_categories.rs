use synd_feed::{
    entry::{Entry, SourceCategory},
    types::FeedType,
};

use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryBatch, Stage},
};

pub(in crate::feed::pipeline) struct SourceCategories {
    pub rss_delimiter: char,
}

impl Default for SourceCategories {
    fn default() -> Self {
        Self { rss_delimiter: ',' }
    }
}

impl Stage for SourceCategories {
    type Input = EntryBatch<Entry>;
    type Output = EntryBatch<Entry>;

    async fn run<Tx: FeedDb + Send>(
        &self,
        mut batch: Self::Input,
        _: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Self::Output> {
        if !matches!(
            batch.meta.r#type(),
            FeedType::RSS0 | FeedType::RSS1 | FeedType::RSS2
        ) {
            return Ok(batch);
        }

        for entry in &mut batch.entries {
            let Some(categories) = entry.source_categories()? else {
                continue;
            };
            let mut terms = Vec::new();
            for category in categories {
                for term in category.term.split(self.rss_delimiter).map(str::trim) {
                    terms.push(SourceCategory {
                        term: term.to_owned(),
                        scheme: category.scheme.clone(),
                        label: category.label.clone(),
                    });
                }
            }
            entry.set_source_categories(terms);
        }
        Ok(batch)
    }
}
