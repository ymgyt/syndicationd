use nutype::nutype;

/// Feed category
#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty, len_char_max = 20),
    derive(Debug, Clone, PartialEq, Eq)
)]
pub struct Category(String);
