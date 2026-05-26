use std::ops::ControlFlow;

use crate::{
    application::{Direction, IndexOutOfRange, Populate},
    ui::widgets::filter::{FilterResult, Filterable},
};

pub(crate) struct FilterableVec<T, F> {
    items: Vec<T>,
    effective_items: Vec<usize>,
    selected_item_index: usize,
    filterer: F,
}

impl<T, F> FilterableVec<T, F>
where
    F: Default,
{
    pub(crate) fn new() -> Self {
        Self::from_filter(F::default())
    }

    pub(crate) fn from_filter(filterer: F) -> Self {
        Self {
            items: Vec::new(),
            effective_items: Vec::new(),
            selected_item_index: 0,
            filterer,
        }
    }
}

impl<T, F> FilterableVec<T, F> {
    pub(crate) fn selected(&self) -> Option<&T> {
        self.effective_items
            .get(self.selected_item_index)
            .map(|&idx| &self.items[idx])
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selected_item_index
    }

    pub(crate) fn len(&self) -> usize {
        self.effective_items.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.effective_items.is_empty()
    }

    pub(crate) fn move_selection(&mut self, direction: Direction) {
        self.selected_item_index = direction.apply(
            self.selected_item_index,
            self.effective_items.len(),
            IndexOutOfRange::Wrapping,
        );
    }
    pub(crate) fn move_first(&mut self) {
        self.selected_item_index = 0;
    }

    pub(crate) fn move_last(&mut self) {
        if !self.effective_items.is_empty() {
            self.selected_item_index = self.effective_items.len() - 1;
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.effective_items
            .iter()
            .map(move |&idx| &self.items[idx])
    }

    pub(crate) fn as_unfiltered_slice(&self) -> &[T] {
        self.items.as_slice()
    }

    pub(crate) fn unfiltered_len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn filter(&self) -> &F {
        &self.filterer
    }
}

impl<T, F> FilterableVec<T, F>
where
    F: Filterable<T>,
{
    pub(crate) fn update(&mut self, populate: Populate, items: Vec<T>) {
        match populate {
            Populate::Append => self.items.extend(items),
            Populate::Replace => self.items = items,
        }
        self.refresh();
    }

    pub(crate) fn with_mut<F2>(&mut self, mut f: F2) -> Option<&T>
    where
        F2: FnMut(&mut T) -> ControlFlow<()>,
    {
        let mut found = None;
        for (idx, item) in self.items.iter_mut().enumerate() {
            match f(item) {
                ControlFlow::Break(()) => {
                    found = Some(idx);
                    break;
                }
                ControlFlow::Continue(()) => (),
            }
        }

        if let Some(idx) = found {
            self.refresh();
            self.items.get(idx)
        } else {
            None
        }
    }

    pub(crate) fn update_filter(&mut self, filterer: F) {
        self.filterer = filterer;
        self.refresh();
    }

    pub(crate) fn with_filter<With>(&mut self, f: With)
    where
        With: FnOnce(&mut F),
    {
        f(&mut self.filterer);
        self.refresh();
    }

    pub(crate) fn retain<C>(&mut self, cond: C)
    where
        C: Fn(&T) -> bool,
    {
        self.items.retain(cond);
        self.refresh();
    }

    pub(crate) fn truncate(&mut self, len: usize) {
        self.items.truncate(len);
        self.refresh();
    }

    pub(crate) fn refresh(&mut self) {
        self.effective_items = self
            .items
            .iter()
            .enumerate()
            .filter(|(_idx, item)| self.filterer.filter(item) == FilterResult::Use)
            .map(|(idx, _)| idx)
            .collect();
        // prevent selection from out of index
        self.selected_item_index = self
            .selected_item_index
            .min(self.effective_items.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct UseAll;

    impl Filterable<i32> for UseAll {
        fn filter(&self, _item: &i32) -> FilterResult {
            FilterResult::Use
        }
    }

    #[derive(Default)]
    struct RejectAll;

    impl Filterable<i32> for RejectAll {
        fn filter(&self, _item: &i32) -> FilterResult {
            FilterResult::Discard
        }
    }

    #[test]
    fn move_last_selects_last_visible_item() {
        let mut items = FilterableVec::from_filter(UseAll);
        items.update(Populate::Replace, vec![1, 2, 3]);

        items.move_last();

        assert_eq!(items.selected(), Some(&3));
        assert_eq!(items.selected_index(), 2);
    }

    #[test]
    fn move_last_handles_empty_visible_items() {
        let mut items = FilterableVec::from_filter(RejectAll);
        items.update(Populate::Replace, vec![1, 2, 3]);

        items.move_last();

        assert_eq!(items.selected(), None);
        assert_eq!(items.selected_index(), 0);
    }
}
