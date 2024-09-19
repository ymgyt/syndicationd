use std::borrow::Cow;

pub(crate) enum Credential<'a> {
    Password(Password<'a>),
}

pub(crate) trait Provider {
    fn credential(&self) -> Credential;
}

pub(crate) struct Password<'a> {
    pub(crate) username: Cow<'a, str>,
    pub(crate) password: Cow<'a, str>,
}
