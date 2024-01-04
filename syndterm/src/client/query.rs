#![allow(clippy::all, warnings)]
pub struct User;
pub mod user {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "User";
    pub const QUERY: &str =
        "query User {\n  subscription {\n    feeds {\n      url\n    }\n  }\n}\n";
    use super::*;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[derive(Serialize)]
    pub struct Variables;
    #[derive(Deserialize, Debug)]
    pub struct ResponseData {
        pub subscription: UserSubscription,
    }
    #[derive(Deserialize, Debug)]
    pub struct UserSubscription {
        pub feeds: Vec<UserSubscriptionFeeds>,
    }
    #[derive(Deserialize, Debug)]
    pub struct UserSubscriptionFeeds {
        pub url: String,
    }
}
impl graphql_client::GraphQLQuery for User {
    type Variables = user::Variables;
    type ResponseData = user::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: user::QUERY,
            operation_name: user::OPERATION_NAME,
        }
    }
}
