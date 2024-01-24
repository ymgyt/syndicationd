#![allow(clippy::all, warnings)]
pub struct SubscribeFeed;
pub mod subscribe_feed {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "SubscribeFeed";
    pub const QUERY : & str = "mutation SubscribeFeed($input: SubscribeFeedInput!) {\n  subscribeFeed(input: $input) {\n    __typename\n    ... on SubscribeFeedSuccess {\n      feed {\n        ...FeedMeta\n      }\n      status {\n        code\n      }\n    }\n    ... on SubscribeFeedError {\n      status {\n        code\n      }\n    }\n  }\n}\n\nfragment FeedMeta on Feed {\n  id\n  title\n  url\n  updated\n  websiteUrl\n  description\n  authors {\n    nodes\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n" ;
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
    type Rfc3339Time = crate::client::scalar::Rfc3339Time;
    #[derive(Debug)]
    pub enum ResponseCode {
        OK,
        UNAUTHORIZED,
        INTERNAL_ERROR,
        Other(String),
    }
    impl ::serde::Serialize for ResponseCode {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_str(match *self {
                ResponseCode::OK => "OK",
                ResponseCode::UNAUTHORIZED => "UNAUTHORIZED",
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
                "UNAUTHORIZED" => Ok(ResponseCode::UNAUTHORIZED),
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
    pub struct FeedMeta {
        pub id: ID,
        pub title: Option<String>,
        pub url: String,
        pub updated: Option<Rfc3339Time>,
        #[serde(rename = "websiteUrl")]
        pub website_url: Option<String>,
        pub description: Option<String>,
        pub authors: FeedMetaAuthors,
        pub links: FeedMetaLinks,
    }
    #[derive(Deserialize, Debug)]
    pub struct FeedMetaAuthors {
        pub nodes: Vec<String>,
    }
    #[derive(Deserialize, Debug)]
    pub struct FeedMetaLinks {
        pub nodes: Vec<FeedMetaLinksNodes>,
    }
    pub type FeedMetaLinksNodes = Link;
    #[derive(Deserialize, Debug)]
    pub struct Link {
        pub href: String,
        pub rel: Option<String>,
        #[serde(rename = "mediaType")]
        pub media_type: Option<String>,
        pub title: Option<String>,
    }
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
        pub feed: SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessFeed,
        pub status: SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessStatus,
    }
    pub type SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessFeed = FeedMeta;
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
