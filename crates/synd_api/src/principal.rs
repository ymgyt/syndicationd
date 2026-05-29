use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug)]
pub enum Principal {
    User(User),
}

impl Principal {
    /// Stable identity of the authenticated principal in the API layer.
    ///
    /// Registry-facing code must convert this into a registry `SubscriberId`
    /// at the API boundary instead of passing `Principal` through.
    pub fn principal_id(&self) -> &str {
        match self {
            Principal::User(User { id, .. }) => id.as_str(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct User {
    id: String,
    #[allow(dead_code)]
    email: String,
}

impl User {
    pub fn local() -> Self {
        User {
            id: String::from("local"),
            email: String::from("local"),
        }
    }

    pub fn from_email(email: impl Into<String>) -> Self {
        let mut s = DefaultHasher::new();
        let email = email.into();

        email.hash(&mut s);
        let id = s.finish();
        let id = format!("{id:016x}");

        User { id, email }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::User;

    #[test]
    fn user_from_email() {
        let u = User::from_email("foo@ymgyt.io");
        assert_eq!(u.id().len(), 16);
        assert_eq!(u.id(), "585779d8c9b2e06d");
    }

    #[test]
    fn local_user() {
        let u = User::local();
        assert_eq!(u.id(), "local");
    }
}
