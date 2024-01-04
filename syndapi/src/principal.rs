use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug)]
pub enum Principal {
    User(User),
}

#[derive(Clone, Debug)]
pub struct User {
    id: String,
    email: String,
}

impl User {
    pub fn from_email(email: impl Into<String>) -> Self {
        let mut s = DefaultHasher::new();
        let email = email.into();

        email.hash(&mut s);
        let id = s.finish();
        let id = format!("{:016x}", id);

        User { id, email }
    }

    pub fn id(&self) -> &str {
        &self.id.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::User;

    #[test]
    fn user_from_email() {
        let u = User::from_email("foo@ymgyt.io");
        assert_eq!(u.id.len(), 16);
        assert_eq!(u.id, "585779d8c9b2e06d");
    }
}
