use synd_registry::{RegistryDbError, RegistryDbResult, crawl::policy::CrawlPolicy};

pub(super) fn encode_crawl_policy_json(policy: CrawlPolicy) -> RegistryDbResult<String> {
    serde_json::to_string(&policy).map_err(RegistryDbError::internal)
}

pub(super) fn decode_crawl_policy_json(policy_json: &str) -> RegistryDbResult<CrawlPolicy> {
    serde_json::from_str(policy_json).map_err(RegistryDbError::internal)
}
