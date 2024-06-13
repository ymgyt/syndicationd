#![allow(clippy::all, warnings)]
pub struct Subscription;
pub mod subscription {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Subscription";
    pub const QUERY : & str = "query Subscription($after: String, $first: Int) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      nodes {\n        ...Feed\n      }\n      pageInfo {\n        ...PageInfo\n      }\n      errors {\n        url\n        errorMessage\n      }\n    }\n  }\n}\n\nfragment Feed on Feed {\n  id\n  type\n  title\n  url\n  updated\n  websiteUrl\n  description\n  generator\n  requirement\n  category\n  entries(first: 10) {\n    nodes {\n      ...EntryMeta\n    }\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n  authors {\n    nodes\n  }\n}\n\nfragment EntryMeta on Entry {\n    title\n    published\n    updated\n    summary\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n\nquery Entries($after: String, $first: Int!) {\n  output: subscription {\n    entries(after: $after, first: $first) {\n      nodes {\n        ...Entry\n      }\n      pageInfo {\n        ...PageInfo\n      }\n    }\n  }\n}\n\nfragment Entry on Entry {\n  title\n  published\n  updated\n  summary\n  websiteUrl\n  feed {\n    ...FeedMeta\n  }\n}\n\nfragment FeedMeta on FeedMeta {\n  title\n  url\n  requirement\n  category\n}\n\nfragment PageInfo on PageInfo {\n  hasNextPage\n  endCursor\n}\n\nquery ExportSubscription($after: String, $first: Int!) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      nodes {\n        title\n        url\n      }\n    }\n  }\n}\n" ;
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
    type FeedUrl = crate::client::scalar::FeedUrl;
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
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct Variables {
        pub after: Option<String>,
        pub first: Option<Int>,
    }
    impl Variables {}
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Feed {
        pub id: ID,
        #[serde(rename = "type")]
        pub type_: FeedType,
        pub title: Option<String>,
        pub url: FeedUrl,
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
    pub struct PageInfo {
        #[serde(rename = "hasNextPage")]
        pub has_next_page: Boolean,
        #[serde(rename = "endCursor")]
        pub end_cursor: Option<String>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ResponseData {
        pub output: SubscriptionOutput,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscriptionOutput {
        pub feeds: SubscriptionOutputFeeds,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscriptionOutputFeeds {
        pub nodes: Vec<SubscriptionOutputFeedsNodes>,
        #[serde(rename = "pageInfo")]
        pub page_info: SubscriptionOutputFeedsPageInfo,
        pub errors: Vec<SubscriptionOutputFeedsErrors>,
    }
    pub type SubscriptionOutputFeedsNodes = Feed;
    pub type SubscriptionOutputFeedsPageInfo = PageInfo;
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct SubscriptionOutputFeedsErrors {
        pub url: FeedUrl,
        #[serde(rename = "errorMessage")]
        pub error_message: String,
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
pub struct Entries;
pub mod entries {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Entries";
    pub const QUERY : & str = "query Subscription($after: String, $first: Int) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      nodes {\n        ...Feed\n      }\n      pageInfo {\n        ...PageInfo\n      }\n      errors {\n        url\n        errorMessage\n      }\n    }\n  }\n}\n\nfragment Feed on Feed {\n  id\n  type\n  title\n  url\n  updated\n  websiteUrl\n  description\n  generator\n  requirement\n  category\n  entries(first: 10) {\n    nodes {\n      ...EntryMeta\n    }\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n  authors {\n    nodes\n  }\n}\n\nfragment EntryMeta on Entry {\n    title\n    published\n    updated\n    summary\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n\nquery Entries($after: String, $first: Int!) {\n  output: subscription {\n    entries(after: $after, first: $first) {\n      nodes {\n        ...Entry\n      }\n      pageInfo {\n        ...PageInfo\n      }\n    }\n  }\n}\n\nfragment Entry on Entry {\n  title\n  published\n  updated\n  summary\n  websiteUrl\n  feed {\n    ...FeedMeta\n  }\n}\n\nfragment FeedMeta on FeedMeta {\n  title\n  url\n  requirement\n  category\n}\n\nfragment PageInfo on PageInfo {\n  hasNextPage\n  endCursor\n}\n\nquery ExportSubscription($after: String, $first: Int!) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      nodes {\n        title\n        url\n      }\n    }\n  }\n}\n" ;
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
    type FeedUrl = crate::client::scalar::FeedUrl;
    type Rfc3339Time = crate::client::scalar::Rfc3339Time;
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
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct Variables {
        pub after: Option<String>,
        pub first: Int,
    }
    impl Variables {}
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Entry {
        pub title: Option<String>,
        pub published: Option<Rfc3339Time>,
        pub updated: Option<Rfc3339Time>,
        pub summary: Option<String>,
        #[serde(rename = "websiteUrl")]
        pub website_url: Option<String>,
        pub feed: EntryFeed,
    }
    pub type EntryFeed = FeedMeta;
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct FeedMeta {
        pub title: Option<String>,
        pub url: FeedUrl,
        pub requirement: Option<Requirement>,
        pub category: Option<Category>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct PageInfo {
        #[serde(rename = "hasNextPage")]
        pub has_next_page: Boolean,
        #[serde(rename = "endCursor")]
        pub end_cursor: Option<String>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ResponseData {
        pub output: EntriesOutput,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct EntriesOutput {
        pub entries: EntriesOutputEntries,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct EntriesOutputEntries {
        pub nodes: Vec<EntriesOutputEntriesNodes>,
        #[serde(rename = "pageInfo")]
        pub page_info: EntriesOutputEntriesPageInfo,
    }
    pub type EntriesOutputEntriesNodes = Entry;
    pub type EntriesOutputEntriesPageInfo = PageInfo;
}
impl graphql_client::GraphQLQuery for Entries {
    type Variables = entries::Variables;
    type ResponseData = entries::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: entries::QUERY,
            operation_name: entries::OPERATION_NAME,
        }
    }
}
pub struct ExportSubscription;
pub mod export_subscription {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "ExportSubscription";
    pub const QUERY : & str = "query Subscription($after: String, $first: Int) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      nodes {\n        ...Feed\n      }\n      pageInfo {\n        ...PageInfo\n      }\n      errors {\n        url\n        errorMessage\n      }\n    }\n  }\n}\n\nfragment Feed on Feed {\n  id\n  type\n  title\n  url\n  updated\n  websiteUrl\n  description\n  generator\n  requirement\n  category\n  entries(first: 10) {\n    nodes {\n      ...EntryMeta\n    }\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n  authors {\n    nodes\n  }\n}\n\nfragment EntryMeta on Entry {\n    title\n    published\n    updated\n    summary\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n\nquery Entries($after: String, $first: Int!) {\n  output: subscription {\n    entries(after: $after, first: $first) {\n      nodes {\n        ...Entry\n      }\n      pageInfo {\n        ...PageInfo\n      }\n    }\n  }\n}\n\nfragment Entry on Entry {\n  title\n  published\n  updated\n  summary\n  websiteUrl\n  feed {\n    ...FeedMeta\n  }\n}\n\nfragment FeedMeta on FeedMeta {\n  title\n  url\n  requirement\n  category\n}\n\nfragment PageInfo on PageInfo {\n  hasNextPage\n  endCursor\n}\n\nquery ExportSubscription($after: String, $first: Int!) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      nodes {\n        title\n        url\n      }\n    }\n  }\n}\n" ;
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
    type FeedUrl = crate::client::scalar::FeedUrl;
    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct Variables {
        pub after: Option<String>,
        pub first: Int,
    }
    impl Variables {}
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ResponseData {
        pub output: ExportSubscriptionOutput,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ExportSubscriptionOutput {
        pub feeds: ExportSubscriptionOutputFeeds,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ExportSubscriptionOutputFeeds {
        #[serde(rename = "pageInfo")]
        pub page_info: ExportSubscriptionOutputFeedsPageInfo,
        pub nodes: Vec<ExportSubscriptionOutputFeedsNodes>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ExportSubscriptionOutputFeedsPageInfo {
        #[serde(rename = "hasNextPage")]
        pub has_next_page: Boolean,
        #[serde(rename = "endCursor")]
        pub end_cursor: Option<String>,
    }
    #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct ExportSubscriptionOutputFeedsNodes {
        pub title: Option<String>,
        pub url: FeedUrl,
    }
}
impl graphql_client::GraphQLQuery for ExportSubscription {
    type Variables = export_subscription::Variables;
    type ResponseData = export_subscription::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: export_subscription::QUERY,
            operation_name: export_subscription::OPERATION_NAME,
        }
    }
}
