#![allow(clippy::all, warnings)]
pub struct Subscription;
pub mod subscription {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Subscription";
    pub const QUERY : & str = "query Subscription($after: String, $first: Int) {\n  output: subscription {\n    feeds(after: $after, first: $first) {\n      nodes {\n        ...FeedMeta\n      }\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n    }\n  }\n}\n\nfragment FeedMeta on Feed {\n  id\n  title\n  url\n  updated\n  websiteUrl\n  description\n  entries(first: 5) {\n    nodes {\n      ...EntryMeta\n    }\n  }\n  links {\n    nodes {\n      ...Link\n    }\n  }\n}\n\nfragment EntryMeta on Entry {\n    title,\n    published,\n    summary,\n}\n\nfragment Link on Link {\n  href\n  rel\n  mediaType\n  title  \n}\n" ;
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
    #[derive(Serialize)]
    pub struct Variables {
        pub after: Option<String>,
        pub first: Option<Int>,
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
        pub entries: FeedMetaEntries,
        pub links: FeedMetaLinks,
    }
    #[derive(Deserialize, Debug)]
    pub struct FeedMetaEntries {
        pub nodes: Vec<FeedMetaEntriesNodes>,
    }
    pub type FeedMetaEntriesNodes = EntryMeta;
    #[derive(Deserialize, Debug)]
    pub struct FeedMetaLinks {
        pub nodes: Vec<FeedMetaLinksNodes>,
    }
    pub type FeedMetaLinksNodes = Link;
    #[derive(Deserialize, Debug)]
    pub struct EntryMeta {
        pub title: Option<String>,
        pub published: Option<Rfc3339Time>,
        pub summary: Option<String>,
    }
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
        pub output: SubscriptionOutput,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscriptionOutput {
        pub feeds: SubscriptionOutputFeeds,
    }
    #[derive(Deserialize, Debug)]
    pub struct SubscriptionOutputFeeds {
        pub nodes: Vec<SubscriptionOutputFeedsNodes>,
        #[serde(rename = "pageInfo")]
        pub page_info: SubscriptionOutputFeedsPageInfo,
    }
    pub type SubscriptionOutputFeedsNodes = FeedMeta;
    #[derive(Deserialize, Debug)]
    pub struct SubscriptionOutputFeedsPageInfo {
        #[serde(rename = "hasNextPage")]
        pub has_next_page: Boolean,
        #[serde(rename = "endCursor")]
        pub end_cursor: Option<String>,
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
