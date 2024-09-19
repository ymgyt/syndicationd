use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub(crate) struct Namespace(Cow<'static, str>);

impl From<&'static str> for Namespace {
    fn from(s: &'static str) -> Self {
        Namespace(Cow::Borrowed(s))
    }
}

impl<'a> From<Cow<'a, str>> for Namespace {
    fn from(s: Cow<'a, str>) -> Self {
        match s {
            Cow::Borrowed(s) => Namespace(Cow::Owned(s.to_owned())),
            Cow::Owned(s) => Namespace(Cow::Owned(s)),
        }
    }
}
