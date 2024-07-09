use crate::ui::components::filter::{FilterResult, Filterable};

#[derive(Default, Debug, Clone)]
pub(crate) struct ComposedFilterer<L, R> {
    left: L,
    right: R,
}

impl<L, R> ComposedFilterer<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left, right }
    }

    pub(crate) fn update_left(&mut self, left: L) {
        self.left = left;
    }

    pub(crate) fn update_right(&mut self, right: R) {
        self.right = right;
    }

    pub(crate) fn and_then<F>(self, right: F) -> ComposedFilterer<Self, F> {
        ComposedFilterer::new(self, right)
    }

    pub(crate) fn right(&self) -> &R {
        &self.right
    }
}

impl<L, R, T> Filterable<T> for ComposedFilterer<L, R>
where
    L: Filterable<T>,
    R: Filterable<T>,
{
    fn filter(&self, item: &T) -> super::FilterResult {
        if self.left.filter(item) == FilterResult::Use
            && self.right.filter(item) == FilterResult::Use
        {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }
}

pub(crate) trait Composable {
    fn and_then<F>(self, right: F) -> ComposedFilterer<Self, F>
    where
        Self: Sized,
    {
        ComposedFilterer::new(self, right)
    }
}
