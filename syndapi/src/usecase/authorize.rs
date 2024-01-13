use std::ops::Deref;

use crate::principal::Principal;

use super::Usecase;

pub struct Authorized<T> {
    principal: T,
}

impl Authorized<Principal> {
    fn new(principal: Principal) -> Self {
        Self { principal }
    }
}

impl<T> Deref for Authorized<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.principal
    }
}

pub struct Unauthorized;

pub struct Authorizer {}

impl Authorizer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn authorize<U: Usecase>(
        &self,
        principal: Principal,
        usecase: &U,
        input: &U::Input,
    ) -> Result<Authorized<Principal>, Unauthorized> {
        usecase
            .authorize(principal, input)
            .await
            .map(Authorized::new)
    }
}
