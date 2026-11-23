#[derive(Debug, Clone, Copy)]
pub(super) struct PageLimit {
    first: usize,
}

impl PageLimit {
    pub(super) fn new(first: usize) -> Self {
        Self { first }
    }

    pub(super) fn sql_limit(self) -> i64 {
        i64::try_from(self.first.saturating_add(1)).unwrap_or(i64::MAX)
    }

    pub(super) fn truncate_overfetch<T>(self, items: &mut Vec<T>) -> bool {
        let has_next_page = items.len() > self.first;
        if has_next_page {
            items.truncate(self.first);
        }
        has_next_page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_limit_fetches_one_extra_item() {
        assert_eq!(PageLimit::new(10).sql_limit(), 11);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn sql_limit_saturates_when_the_extra_item_cannot_be_represented() {
        assert_eq!(PageLimit::new(usize::MAX).sql_limit(), i64::MAX);
    }

    #[test]
    fn truncate_overfetch_reports_next_page_and_removes_extra_item() {
        let mut items = vec![1, 2, 3];

        let has_next_page = PageLimit::new(2).truncate_overfetch(&mut items);

        assert!(has_next_page);
        assert_eq!(items, vec![1, 2]);
    }

    #[test]
    fn truncate_overfetch_keeps_exact_page() {
        let mut items = vec![1, 2];

        let has_next_page = PageLimit::new(2).truncate_overfetch(&mut items);

        assert!(!has_next_page);
        assert_eq!(items, vec![1, 2]);
    }
}
