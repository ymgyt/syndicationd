#![allow(clippy::all, warnings)]
pub struct Subscription;
pub mod subscription {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Subscription";
    pub const QUERY : & str = "query Subscription {\n  output: subscription {\n    feeds {\n      nodes {\n        url\n        title\n      }\n    }\n  }\n}\n" ;
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
        pub output: SubscriptionOutput,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscriptionOutput {
        pub feeds: SubscriptionOutputFeeds,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscriptionOutputFeeds {
        pub nodes: Vec<SubscriptionOutputFeedsNodes>,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscriptionOutputFeedsNodes {
        pub url: String,
        pub title: String,
    }
}
impl graphql_client::GraphQLQuery for Subscription {
    type Variables = subscription::Variables;
    type ResponseData = subscription::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: subscription::QUERY,
            operation_name: subscription::OPERATION_NAME,
        }
    }
}
