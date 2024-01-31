#![allow(clippy::all, warnings)]
pub struct Authenticate;
pub mod authenticate {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "Authenticate";
    pub const QUERY: &str = "query Authenticate {\n  viewer {\n    email,\n  }\n}\n";
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
        pub viewer: AuthenticateViewer,
    }
    #[derive(Deserialize, Debug)]
    pub struct AuthenticateViewer {
        pub email: String,
    }
}
impl graphql_client::GraphQLQuery for Authenticate {
    type Variables = authenticate::Variables;
    type ResponseData = authenticate::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: authenticate::QUERY,
            operation_name: authenticate::OPERATION_NAME,
        }
    }
}
