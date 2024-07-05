use crate::{
    application::{Direction, IndexOutOfRange, Populate},
    ui::components::filter::{FilterResult, Filterable},
};

pub(crate) struct FilterableVec<T, F> {
    pub(crate) items: Vec<T>,
    effective_items: Vec<usize>,
    selected_item_index: usize,
    filterer: F,
}

impl<T, F> FilterableVec<T, F>
where
    F: Default,
{
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            effective_items: Vec::new(),
            selected_item_index: 0,
            filterer: F::default(),
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
        if !self.items.is_empty() {
            self.selected_item_index = self.effective_items.len() - 1;
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.effective_items
            .iter()
            .map(move |&idx| &self.items[idx])
    }

    pub(crate) fn unfiltered_iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut()
    }

    pub(crate) fn as_unfiltered_slice(&self) -> &[T] {
        self.items.as_slice()
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

    pub(crate) fn upsert_first<C>(&mut self, item: T, should_update: C)
    where
        C: Fn(&T) -> bool,
    {
        match self.items.iter_mut().find(|item| should_update(item)) {
            Some(old) => *old = item,
            None => self.items.insert(0, item),
        }
        self.refresh();
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

    fn refresh(&mut self) {
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
