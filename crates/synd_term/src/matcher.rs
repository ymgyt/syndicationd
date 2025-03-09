use std::{cell::RefCell, fmt, rc::Rc};

use nucleo::{
    Utf32Str,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

#[derive(Clone)]
pub struct Matcher {
    matcher: Rc<RefCell<nucleo::Matcher>>,
    needle: Option<Pattern>,
    // For Utf32 conversion
    buf: Rc<RefCell<Vec<char>>>,
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            matcher: Rc::new(RefCell::new(nucleo::Matcher::default())),
            needle: None,
            buf: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn update_needle(&mut self, needle: &str) {
        if needle.is_empty() {
            self.needle = None;
        } else {
            let needle = Pattern::new(
                needle,
                CaseMatching::Smart,
                Normalization::Smart,
                AtomKind::Substring,
            );
            self.needle = Some(needle);
        }
    }

    pub fn r#match(&self, haystack: &str) -> bool {
        match self.needle.as_ref() {
            Some(needle) => {
                let mut buf = self.buf.borrow_mut();
                let haystack = Utf32Str::new(haystack, &mut buf);
                needle
                    .score(haystack, &mut self.matcher.borrow_mut())
                    .unwrap_or(0)
                    > 0
            }
            None => true,
        }
    }
}

impl fmt::Debug for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Matcher")
            .field("needle", &self.needle)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_match() {
        let mut m = Matcher::new();

        let cases = vec![
            ("rustsec", "rustsec"),
            ("rustsec", "RUSTSEC"),
            ("rustsec", "RustSec"),
            ("this week in rust", "This Week in Rust"),
        ];

        for case in cases {
            m.update_needle(case.0);
            assert!(m.r#match(case.1));
        }
    }
}
