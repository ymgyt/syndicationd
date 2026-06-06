use std::path::{Path, PathBuf};

use synd_support::dirs::SyndicationdDirs;

use crate::{
    Result, RuntimeConfig, RuntimeDatabase, instance::RuntimeInstance, startup::StartupLockPath,
    uds::UdsEndpoint,
};

pub(crate) const RUNTIME_ROOT_ENV: &str = "SYND_RUNTIME_ROOT";

/// Filesystem root used to place runtime implementation artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeRoot {
    path: PathBuf,
}

impl RuntimeRoot {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl From<PathBuf> for RuntimeRoot {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl From<&Path> for RuntimeRoot {
    fn from(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

/// Environment-derived values used to resolve runtime placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimePlacementEnvironment {
    default_root: RuntimeRoot,
}

impl RuntimePlacementEnvironment {
    pub(crate) fn capture() -> Self {
        if let Some(root) = std::env::var_os(RUNTIME_ROOT_ENV)
            && !root.as_os_str().is_empty()
        {
            return Self::from_root(root);
        }

        Self {
            default_root: RuntimeRoot::from(SyndicationdDirs::current().runtime_dir_or_temp()),
        }
    }

    pub(crate) fn from_root(root: impl Into<PathBuf>) -> Self {
        Self {
            default_root: RuntimeRoot::from(root.into()),
        }
    }

    #[cfg(test)]
    pub(crate) fn new(default_root: RuntimeRoot) -> Self {
        Self { default_root }
    }

    pub(crate) fn default_root(&self) -> RuntimeRoot {
        self.default_root.clone()
    }
}

/// Resolves runtime configuration into concrete placement.
#[derive(Debug, Clone)]
pub(crate) struct RuntimePlacementResolver {
    environment: RuntimePlacementEnvironment,
}

impl RuntimePlacementResolver {
    pub(crate) fn with_environment(environment: RuntimePlacementEnvironment) -> Self {
        Self { environment }
    }

    pub(crate) fn resolve(&self, config: &RuntimeConfig) -> Result<RuntimePlacement> {
        self.resolve_database(config.database())
    }

    pub(crate) fn resolve_database(&self, database: &RuntimeDatabase) -> Result<RuntimePlacement> {
        let instance = RuntimeInstance::from_database(database)?;

        Ok(RuntimePlacement::from_instance(
            self.environment.default_root(),
            instance,
        ))
    }
}

/// Filesystem and transport placement for a resolved `RuntimeInstance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimePlacement {
    root: RuntimeRoot,
    instance: RuntimeInstance,
    endpoint: UdsEndpoint,
    startup_lock_path: StartupLockPath,
}

impl RuntimePlacement {
    pub(crate) fn from_instance(root: RuntimeRoot, instance: RuntimeInstance) -> Self {
        let endpoint = UdsEndpoint::from_instance_id(root.path(), instance.id());
        let startup_lock_path = StartupLockPath::from_instance_id(root.path(), instance.id());

        Self {
            root,
            instance,
            endpoint,
            startup_lock_path,
        }
    }

    pub(crate) fn root(&self) -> &RuntimeRoot {
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
}

#[cfg(test)]
mod tests {
    use crate::{
        RuntimeConfig, RuntimeDatabase,
        instance::RuntimeInstance,
        placement::{
            RuntimePlacement, RuntimePlacementEnvironment, RuntimePlacementResolver, RuntimeRoot,
        },
    };

    mod from_instance {
        use super::*;

        #[test]
        fn derives_paths() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let root = RuntimeRoot::from(tmp.path().join("runtime"));
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();

            let placement = RuntimePlacement::from_instance(root.clone(), instance.clone());

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
        }
    }

    mod resolver {
        use super::*;

        #[test]
        fn uses_environment() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let default_root = RuntimeRoot::from(tmp.path().join("runtime"));
            let environment = RuntimePlacementEnvironment::new(default_root.clone());
            let resolver = RuntimePlacementResolver::with_environment(environment);
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
        }
    }
}
