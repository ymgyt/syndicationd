#![allow(clippy::all, warnings)]
pub struct SubscribeFeed;
pub mod subscribe_feed {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "SubscribeFeed";
    pub const QUERY : & str = "mutation SubscribeFeed($input: SubscribeFeedInput!) {\n  subscribeFeed(input: $input) {\n    __typename\n    ... on SubscribeFeedSuccess {\n      url\n      status {\n        code\n      }\n    }\n    ... on SubscribeFeedError {\n      status {\n        code\n      }\n    }\n  }\n}\n" ;
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
    #[derive(Debug)]
    pub enum ResponseCode {
        OK,
        INTERNAL_ERROR,
        Other(String),
    }
    impl ::serde::Serialize for ResponseCode {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_str(match *self {
                ResponseCode::OK => "OK",
                ResponseCode::INTERNAL_ERROR => "INTERNAL_ERROR",
                ResponseCode::Other(ref s) => &s,
            })
        }
    }
    impl<'de> ::serde::Deserialize<'de> for ResponseCode {
        fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let s: String = ::serde::Deserialize::deserialize(deserializer)?;
            match s.as_str() {
                "OK" => Ok(ResponseCode::OK),
                "INTERNAL_ERROR" => Ok(ResponseCode::INTERNAL_ERROR),
                _ => Ok(ResponseCode::Other(s)),
            }
        }
    }
    #[derive(Serialize)]
    pub struct SubscribeFeedInput {
        pub url: String,
    }
    #[derive(Serialize)]
    pub struct Variables {
        pub input: SubscribeFeedInput,
    }
    impl Variables {}
    #[derive(Deserialize, Debug)]
    pub struct ResponseData {
        #[serde(rename = "subscribeFeed")]
        pub subscribe_feed: SubscribeFeedSubscribeFeed,
    }
    #[derive(Deserialize, Debug)]
    #[serde(tag = "__typename")]
    pub enum SubscribeFeedSubscribeFeed {
        SubscribeFeedSuccess(SubscribeFeedSubscribeFeedOnSubscribeFeedSuccess),
        SubscribeFeedError(SubscribeFeedSubscribeFeedOnSubscribeFeedError),
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedSuccess {
        pub url: String,
        pub status: SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessStatus,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessStatus {
        pub code: ResponseCode,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedError {
        pub status: SubscribeFeedSubscribeFeedOnSubscribeFeedErrorStatus,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedErrorStatus {
        pub code: ResponseCode,
    }
}
impl graphql_client::GraphQLQuery for SubscribeFeed {
    type Variables = subscribe_feed::Variables;
    type ResponseData = subscribe_feed::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: subscribe_feed::QUERY,
            operation_name: subscribe_feed::OPERATION_NAME,
        }
    }
}
