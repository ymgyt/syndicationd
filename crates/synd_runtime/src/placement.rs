use std::path::{Path, PathBuf};

use synd_support::dirs::SyndicationdDirs;

use crate::{
    Result, RuntimeConfig, RuntimeDatabase,
    instance::{RuntimeInstance, RuntimeInstanceId},
    startup::StartupLockPath,
    uds::UdsEndpoint,
};

pub(crate) const RUNTIME_ROOT_ENV: &str = "SYND_RUNTIME_ROOT";

/// Filesystem root used to place runtime implementation artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlacementRoot {
    path: PathBuf,
}

impl PlacementRoot {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl From<PathBuf> for PlacementRoot {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl From<&Path> for PlacementRoot {
    fn from(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

/// Environment-derived values used to resolve runtime placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlacementEnvironment {
    default_root: PlacementRoot,
}

impl PlacementEnvironment {
    pub(crate) fn capture() -> Self {
        if let Some(root) = std::env::var_os(RUNTIME_ROOT_ENV)
            && !root.as_os_str().is_empty()
        {
            return Self::from_root(root);
        }

        Self {
            default_root: PlacementRoot::from(SyndicationdDirs::current().runtime_dir_or_temp()),
        }
    }

    pub(crate) fn from_root(root: impl Into<PathBuf>) -> Self {
        Self {
            default_root: PlacementRoot::from(root.into()),
        }
    }

    #[cfg(test)]
    pub(crate) fn new(default_root: PlacementRoot) -> Self {
        Self { default_root }
    }

    pub(crate) fn default_root(&self) -> PlacementRoot {
        self.default_root.clone()
    }
}

/// Resolves runtime configuration into concrete placement.
#[derive(Debug, Clone)]
pub(crate) struct PlacementResolver {
    environment: PlacementEnvironment,
}

impl PlacementResolver {
    pub(crate) fn with_environment(environment: PlacementEnvironment) -> Self {
        Self { environment }
    }

    pub(crate) fn resolve(&self, config: &RuntimeConfig) -> Result<PlacementSpec> {
        self.resolve_database(config.database())
    }

    pub(crate) fn resolve_database(&self, database: &RuntimeDatabase) -> Result<PlacementSpec> {
        let instance = RuntimeInstance::from_database(database)?;

        Ok(PlacementSpec::from_instance(
            self.environment.default_root(),
            instance,
        ))
    }
}

/// Filesystem and transport placement for a resolved `RuntimeInstance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlacementSpec {
    root: PlacementRoot,
    instance: RuntimeInstance,
    endpoint: UdsEndpoint,
    startup_lock_path: StartupLockPath,
    daemon_claim_path: DaemonClaimPath,
    daemon_claim_lock_path: DaemonClaimLockPath,
}

impl PlacementSpec {
    pub(crate) fn from_instance(root: PlacementRoot, instance: RuntimeInstance) -> Self {
        let endpoint = UdsEndpoint::from_instance_id(root.path(), instance.id());
        let startup_lock_path = StartupLockPath::from_instance_id(root.path(), instance.id());
        let daemon_claim_path = DaemonClaimPath::from_instance_id(root.path(), instance.id());
        let daemon_claim_lock_path =
            DaemonClaimLockPath::from_instance_id(root.path(), instance.id());

        Self {
            root,
            instance,
            endpoint,
            startup_lock_path,
            daemon_claim_path,
            daemon_claim_lock_path,
        }
    }

    pub(crate) fn root(&self) -> &PlacementRoot {
        &self.root
    }

    pub(crate) fn instance(&self) -> &RuntimeInstance {
        &self.instance
    }

    pub(crate) fn endpoint(&self) -> &UdsEndpoint {
        &self.endpoint
    }

    pub(crate) fn startup_lock_path(&self) -> &StartupLockPath {
        &self.startup_lock_path
    }

    pub(crate) fn daemon_claim_path(&self) -> &DaemonClaimPath {
        &self.daemon_claim_path
    }

    pub(crate) fn daemon_claim_lock_path(&self) -> &DaemonClaimLockPath {
        &self.daemon_claim_lock_path
    }
}

/// Filesystem path for a daemon's persisted self-claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DaemonClaimPath {
    path: PathBuf,
}

impl DaemonClaimPath {
    pub(crate) fn from_instance_id(root_dir: &Path, instance_id: &RuntimeInstanceId) -> Self {
        Self {
            path: root_dir.join(format!("api-{instance_id}.claim.json")),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Filesystem path for the lock held while a daemon claim is live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DaemonClaimLockPath {
    path: PathBuf,
}

impl DaemonClaimLockPath {
    pub(crate) fn from_instance_id(root_dir: &Path, instance_id: &RuntimeInstanceId) -> Self {
        Self {
            path: root_dir.join(format!("api-{instance_id}.claim.lock")),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        RuntimeConfig, RuntimeDatabase,
        instance::RuntimeInstance,
        placement::{PlacementEnvironment, PlacementResolver, PlacementRoot, PlacementSpec},
    };

    mod from_instance {
        use super::*;

        #[test]
        fn derives_paths() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let root = PlacementRoot::from(tmp.path().join("runtime"));
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();

            let placement = PlacementSpec::from_instance(root.clone(), instance.clone());

            assert_eq!(placement.root(), &root);
            assert_eq!(placement.instance(), &instance);
            assert_eq!(
                placement.endpoint().path(),
                root.path().join(format!("api-{}.sock", instance.id()))
            );
            assert_eq!(
                placement.startup_lock_path().path(),
                root.path().join(format!("api-{}.lock", instance.id()))
            );
            assert_eq!(
                placement.daemon_claim_path().path(),
                root.path()
                    .join(format!("api-{}.claim.json", instance.id()))
            );
            assert_eq!(
                placement.daemon_claim_lock_path().path(),
                root.path()
                    .join(format!("api-{}.claim.lock", instance.id()))
            );
        }
    }

    mod resolver {
        use super::*;

        #[test]
        fn uses_environment() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let default_root = PlacementRoot::from(tmp.path().join("runtime"));
            let environment = PlacementEnvironment::new(default_root.clone());
            let resolver = PlacementResolver::with_environment(environment);
            let config = RuntimeConfig::new(RuntimeDatabase::sqlite(&db));

            let placement = resolver.resolve(&config).unwrap();

            assert_eq!(placement.root(), &default_root);
            assert_eq!(
                placement.instance().canonical_database_path(),
                tmp.path().canonicalize().unwrap().join("synd.db")
            );
            assert_eq!(
                placement.endpoint().path(),
                default_root
                    .path()
                    .join(format!("api-{}.sock", placement.instance().id()))
            );
            assert_eq!(
                placement.startup_lock_path().path(),
                default_root
                    .path()
                    .join(format!("api-{}.lock", placement.instance().id()))
            );
            assert_eq!(
                placement.daemon_claim_path().path(),
                default_root
                    .path()
                    .join(format!("api-{}.claim.json", placement.instance().id()))
            );
            assert_eq!(
                placement.daemon_claim_lock_path().path(),
                default_root
                    .path()
                    .join(format!("api-{}.claim.lock", placement.instance().id()))
            );
        }
    }
}
