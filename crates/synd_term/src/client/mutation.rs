#![allow(clippy::all, warnings)]
pub struct SubscribeFeed;
pub mod subscribe_feed {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "SubscribeFeed";
    pub const QUERY : & str = "mutation SubscribeFeed($subscribeInput: SubscribeFeedInput!) {\n  subscribeFeed(input: $subscribeInput) {\n    __typename\n    ... on SubscribeFeedSuccess {\n      feed {\n        ...Feed\n      }\n      status {\n        code\n      }\n    }\n    ... on SubscribeFeedError {\n      status {\n        code\n      }\n      message\n    }\n  }\n}\n\nmutation UnsubscribeFeed($unsubscribeInput: UnsubscribeFeedInput!) {\n  unsubscribeFeed(input: $unsubscribeInput) {\n    __typename\n    ... on UnsubscribeFeedSuccess {\n      status {\n        code\n      }\n    }\n    ... on UnsubscribeFeedError {\n      status {\n        code\n      }\n    }\n  }\n}\n\nfragment Feed on Feed {\n  id\n  type\n  title\n  url\n  updated\n  websiteUrl\n  description\n  generator\n  requirement\n  category\n  entries(first: 20) {\n    nodes {\n      ...EntryMeta\n    }\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n  authors {\n    nodes\n  }\n}\n\nfragment EntryMeta on Entry {\n    title,\n    published,\n    updated,\n    summary,\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n" ;
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
    type Category = crate::client::scalar::Category;
    type Rfc3339Time = crate::client::scalar::Rfc3339Time;
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum FeedType {
        ATOM,
        RSS1,
        RSS2,
        RSS0,
        JSON,
        Other(String),
    }
    impl ::serde::Serialize for FeedType {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_str(match *self {
                FeedType::ATOM => "ATOM",
                FeedType::RSS1 => "RSS1",
                FeedType::RSS2 => "RSS2",
                FeedType::RSS0 => "RSS0",
                FeedType::JSON => "JSON",
                FeedType::Other(ref s) => &s,
            })
        }
    }
    impl<'de> ::serde::Deserialize<'de> for FeedType {
        fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let s: String = ::serde::Deserialize::deserialize(deserializer)?;
            match s.as_str() {
                "ATOM" => Ok(FeedType::ATOM),
                "RSS1" => Ok(FeedType::RSS1),
                "RSS2" => Ok(FeedType::RSS2),
                "RSS0" => Ok(FeedType::RSS0),
                "JSON" => Ok(FeedType::JSON),
                _ => Ok(FeedType::Other(s)),
            }
        }
    }
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum Requirement {
        MUST,
        SHOULD,
        MAY,
        Other(String),
    }
    impl ::serde::Serialize for Requirement {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_str(match *self {
                Requirement::MUST => "MUST",
                Requirement::SHOULD => "SHOULD",
                Requirement::MAY => "MAY",
                Requirement::Other(ref s) => &s,
            })
        }
    }
    impl<'de> ::serde::Deserialize<'de> for Requirement {
        fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let s: String = ::serde::Deserialize::deserialize(deserializer)?;
            match s.as_str() {
                "MUST" => Ok(Requirement::MUST),
                "SHOULD" => Ok(Requirement::SHOULD),
                "MAY" => Ok(Requirement::MAY),
                _ => Ok(Requirement::Other(s)),
            }
        }
    }
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum ResponseCode {
        OK,
        UNAUTHORIZED,
        INVALID_FEED_URL,
        INTERNAL_ERROR,
        Other(String),
    }
    impl ::serde::Serialize for ResponseCode {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_str(match *self {
                ResponseCode::OK => "OK",
                ResponseCode::UNAUTHORIZED => "UNAUTHORIZED",
                ResponseCode::INVALID_FEED_URL => "INVALID_FEED_URL",
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
                "INVALID_FEED_URL" => Ok(ResponseCode::INVALID_FEED_URL),
                "INTERNAL_ERROR" => Ok(ResponseCode::INTERNAL_ERROR),
                _ => Ok(ResponseCode::Other(s)),
            }
        }
    }
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscribeFeedInput {
        pub url: String,
        pub requirement: Option<Requirement>,
        pub category: Option<Category>,
    }
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct Variables {
        #[serde(rename = "subscribeInput")]
        pub subscribe_input: SubscribeFeedInput,
    }
    impl Variables {}
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Feed {
        pub id: ID,
        #[serde(rename = "type")]
        pub type_: FeedType,
        pub title: Option<String>,
        pub url: String,
        pub updated: Option<Rfc3339Time>,
        #[serde(rename = "websiteUrl")]
        pub website_url: Option<String>,
        pub description: Option<String>,
        pub generator: Option<String>,
        pub requirement: Option<Requirement>,
        pub category: Option<Category>,
        pub entries: FeedEntries,
        pub links: FeedLinks,
        pub authors: FeedAuthors,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct FeedEntries {
        pub nodes: Vec<FeedEntriesNodes>,
    }
    pub type FeedEntriesNodes = EntryMeta;
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct FeedLinks {
        pub nodes: Vec<FeedLinksNodes>,
    }
    pub type FeedLinksNodes = Link;
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct FeedAuthors {
        pub nodes: Vec<String>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct EntryMeta {
        pub title: Option<String>,
        pub published: Option<Rfc3339Time>,
        pub updated: Option<Rfc3339Time>,
        pub summary: Option<String>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Link {
        pub href: String,
        pub rel: Option<String>,
        #[serde(rename = "mediaType")]
        pub media_type: Option<String>,
        pub title: Option<String>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ResponseData {
        #[serde(rename = "subscribeFeed")]
        pub subscribe_feed: SubscribeFeedSubscribeFeed,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    #[serde(tag = "__typename")]
    pub enum SubscribeFeedSubscribeFeed {
        SubscribeFeedSuccess(SubscribeFeedSubscribeFeedOnSubscribeFeedSuccess),
        SubscribeFeedError(SubscribeFeedSubscribeFeedOnSubscribeFeedError),
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedSuccess {
        pub feed: SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessFeed,
        pub status: SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessStatus,
    }
    pub type SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessFeed = Feed;
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedSuccessStatus {
        pub code: ResponseCode,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscribeFeedSubscribeFeedOnSubscribeFeedError {
        pub status: SubscribeFeedSubscribeFeedOnSubscribeFeedErrorStatus,
        pub message: String,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
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
pub struct UnsubscribeFeed;
pub mod unsubscribe_feed {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "UnsubscribeFeed";
    pub const QUERY : & str = "mutation SubscribeFeed($subscribeInput: SubscribeFeedInput!) {\n  subscribeFeed(input: $subscribeInput) {\n    __typename\n    ... on SubscribeFeedSuccess {\n      feed {\n        ...Feed\n      }\n      status {\n        code\n      }\n    }\n    ... on SubscribeFeedError {\n      status {\n        code\n      }\n      message\n    }\n  }\n}\n\nmutation UnsubscribeFeed($unsubscribeInput: UnsubscribeFeedInput!) {\n  unsubscribeFeed(input: $unsubscribeInput) {\n    __typename\n    ... on UnsubscribeFeedSuccess {\n      status {\n        code\n      }\n    }\n    ... on UnsubscribeFeedError {\n      status {\n        code\n      }\n    }\n  }\n}\n\nfragment Feed on Feed {\n  id\n  type\n  title\n  url\n  updated\n  websiteUrl\n  description\n  generator\n  requirement\n  category\n  entries(first: 20) {\n    nodes {\n      ...EntryMeta\n    }\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n  authors {\n    nodes\n  }\n}\n\nfragment EntryMeta on Entry {\n    title,\n    published,\n    updated,\n    summary,\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n" ;
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
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum ResponseCode {
        OK,
        UNAUTHORIZED,
        INVALID_FEED_URL,
        INTERNAL_ERROR,
        Other(String),
    }
    impl ::serde::Serialize for ResponseCode {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_str(match *self {
                ResponseCode::OK => "OK",
                ResponseCode::UNAUTHORIZED => "UNAUTHORIZED",
                ResponseCode::INVALID_FEED_URL => "INVALID_FEED_URL",
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
                "INVALID_FEED_URL" => Ok(ResponseCode::INVALID_FEED_URL),
                "INTERNAL_ERROR" => Ok(ResponseCode::INTERNAL_ERROR),
                _ => Ok(ResponseCode::Other(s)),
            }
        }
    }
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct UnsubscribeFeedInput {
        pub url: String,
    }
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct Variables {
        #[serde(rename = "unsubscribeInput")]
        pub unsubscribe_input: UnsubscribeFeedInput,
    }
    impl Variables {}
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ResponseData {
        #[serde(rename = "unsubscribeFeed")]
        pub unsubscribe_feed: UnsubscribeFeedUnsubscribeFeed,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    #[serde(tag = "__typename")]
    pub enum UnsubscribeFeedUnsubscribeFeed {
        UnsubscribeFeedSuccess(UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedSuccess),
        UnsubscribeFeedError(UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedError),
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedSuccess {
        pub status: UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedSuccessStatus,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedSuccessStatus {
        pub code: ResponseCode,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedError {
        pub status: UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedErrorStatus,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct UnsubscribeFeedUnsubscribeFeedOnUnsubscribeFeedErrorStatus {
        pub code: ResponseCode,
    }
}
impl graphql_client::GraphQLQuery for UnsubscribeFeed {
    type Variables = unsubscribe_feed::Variables;
    type ResponseData = unsubscribe_feed::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: unsubscribe_feed::QUERY,
            operation_name: unsubscribe_feed::OPERATION_NAME,
        }
    }
}
