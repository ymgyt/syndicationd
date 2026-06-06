use std::{
    fs::{File, OpenOptions},
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(target_os = "linux")]
use rustix::process::{PidfdFlags, pidfd_open, pidfd_send_signal};
use rustix::{
    fs::{FlockOperation, flock},
    io::Errno,
    process::{Pid, Signal, getpgid, getpid, kill_process, test_kill_process},
};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{
    Error, Result,
    placement::{DaemonClaimLockPath, DaemonClaimPath, PlacementSpec},
};

const CLAIM_FORMAT_VERSION: u32 = 1;
const FORCE_WAIT_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DaemonClaim {
    format_version: u32,
    pid: u32,
    process_group_id: Option<u32>,
    runtime_instance_id: String,
    canonical_database_path: PathBuf,
    endpoint_path: PathBuf,
    executable_path: PathBuf,
    process_start_time: Option<u64>,
}

impl DaemonClaim {
    pub(crate) fn for_current_process(placement: &PlacementSpec) -> Result<Self> {
        let pid = getpid();
        let process_group_id = getpgid(None).ok().map(pid_to_u32);

        Ok(Self {
            format_version: CLAIM_FORMAT_VERSION,
            pid: pid_to_u32(pid),
            process_group_id,
            runtime_instance_id: placement.instance().id().to_string(),
            canonical_database_path: placement.instance().canonical_database_path().to_path_buf(),
            endpoint_path: placement.endpoint().path().to_path_buf(),
            executable_path: std::env::current_exe()?,
            process_start_time: process_start_time_for_current_process(),
        })
    }

    pub(crate) fn read(path: &DaemonClaimPath) -> Result<Option<Self>> {
        match std::fs::read(path.path()) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write(&self, path: &DaemonClaimPath) -> Result<()> {
        if let Some(parent) = path.path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let tmp_path = path.path().with_extension("claim.json.tmp");
        std::fs::write(&tmp_path, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp_path, path.path())?;

        Ok(())
    }

    fn remove(path: &DaemonClaimPath) -> Result<()> {
        match std::fs::remove_file(path.path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn validate_placement(&self, placement: &PlacementSpec) -> Result<()> {
        if self.format_version != CLAIM_FORMAT_VERSION {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim format version {} is unsupported",
                    self.format_version
                ),
            });
        }

        let expected_instance_id = placement.instance().id().to_string();
        if self.runtime_instance_id != expected_instance_id {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim runtime instance id {} does not match expected {expected_instance_id}",
                    self.runtime_instance_id
                ),
            });
        }

        if self.canonical_database_path != placement.instance().canonical_database_path() {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim database {} does not match expected {}",
                    self.canonical_database_path.display(),
                    placement.instance().canonical_database_path().display()
                ),
            });
        }

        if self.endpoint_path != placement.endpoint().path() {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim endpoint {} does not match expected {}",
                    self.endpoint_path.display(),
                    placement.endpoint().path().display()
                ),
            });
        }

        Ok(())
    }

    pub(crate) fn pid(&self) -> u32 {
        self.pid
    }
}

#[derive(Debug)]
pub(crate) struct DaemonClaimOwner {
    path: DaemonClaimPath,
    _lock: DaemonClaimLock,
}

impl DaemonClaimOwner {
    pub(crate) fn create(placement: &PlacementSpec) -> Result<Self> {
        let lock =
            match DaemonClaimLockAcquirer::new(placement.daemon_claim_lock_path()).try_acquire()? {
                DaemonClaimLockAcquisition::Acquired(lock) => lock,
                DaemonClaimLockAcquisition::AlreadyHeld => {
                    return Err(Error::DaemonClaimLockAlreadyHeld {
                        path: placement.daemon_claim_lock_path().path().to_path_buf(),
                    });
                }
                #[cfg(not(unix))]
                DaemonClaimLockAcquisition::UnsupportedTransport => {
                    return Err(Error::UnsupportedTransport {
                        context: "daemon claim lock",
                    });
                }
            };

        let claim = DaemonClaim::for_current_process(placement)?;
        claim.write(placement.daemon_claim_path())?;
        debug!(
            daemon_claim = %placement.daemon_claim_path().path().display(),
            pid = claim.pid,
            "Wrote daemon claim"
        );

        Ok(Self {
            path: placement.daemon_claim_path().clone(),
            _lock: lock,
        })
    }
}

impl Drop for DaemonClaimOwner {
    fn drop(&mut self) {
        if let Err(error) = DaemonClaim::remove(&self.path) {
            warn!(
                daemon_claim = %self.path.path().display(),
                error = %error,
                "Failed to remove daemon claim"
            );
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DaemonClaimLockAcquirer {
    path: DaemonClaimLockPath,
}

impl DaemonClaimLockAcquirer {
    pub(crate) fn new(path: &DaemonClaimLockPath) -> Self {
        Self { path: path.clone() }
    }

    pub(crate) fn try_acquire(&self) -> Result<DaemonClaimLockAcquisition> {
        #[cfg(unix)]
        {
            self.create_parent_dir()?;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(self.path.path())?;

            match flock(&file, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => Ok(DaemonClaimLockAcquisition::Acquired(DaemonClaimLock::new(
                    self.path.clone(),
                    file,
                ))),
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    Ok(DaemonClaimLockAcquisition::AlreadyHeld)
                }
                Err(error) => Err(std::io::Error::from(error).into()),
            }
        }

        #[cfg(not(unix))]
        {
            Ok(DaemonClaimLockAcquisition::UnsupportedTransport)
        }
    }

    pub(crate) fn is_held(&self) -> Result<bool> {
        match self.try_acquire()? {
            DaemonClaimLockAcquisition::Acquired(lock) => {
                drop(lock);
                Ok(false)
            }
            DaemonClaimLockAcquisition::AlreadyHeld => Ok(true),
            #[cfg(not(unix))]
            DaemonClaimLockAcquisition::UnsupportedTransport => Err(Error::UnsupportedTransport {
                context: "daemon claim lock",
            }),
        }
    }

    fn create_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum DaemonClaimLockAcquisition {
    Acquired(DaemonClaimLock),
    AlreadyHeld,
    #[cfg(not(unix))]
    UnsupportedTransport,
}

#[derive(Debug)]
pub(crate) struct DaemonClaimLock {
    path: DaemonClaimLockPath,
    file: File,
}

impl DaemonClaimLock {
    fn new(path: DaemonClaimLockPath, file: File) -> Self {
        Self { path, file }
    }
}

impl Drop for DaemonClaimLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Err(error) = flock(&self.file, FlockOperation::Unlock) {
            warn!(
                daemon_claim_lock = %self.path.path().display(),
                error = %error,
                "Failed to unlock daemon claim lock"
            );
        }
    }
}

#[derive(Debug)]
pub(crate) struct SignalTarget {
    pid: Pid,
    #[cfg(target_os = "linux")]
    pidfd: Option<rustix::fd::OwnedFd>,
}

impl SignalTarget {
    pub(crate) fn validate(placement: &PlacementSpec, claim: &DaemonClaim) -> Result<Self> {
        claim.validate_placement(placement)?;

        let Some(pid) =
            Pid::from_raw(
                i32::try_from(claim.pid).map_err(|_| Error::ForceShutdownRefused {
                    reason: format!("daemon claim pid {} is out of range", claim.pid),
                })?,
            )
        else {
            return Err(Error::ForceShutdownRefused {
                reason: "daemon claim pid must be positive".to_owned(),
            });
        };

        let Some(observation) = ProcessObservation::observe(pid)? else {
            return Err(Error::ForceShutdownRefused {
                reason: format!("daemon claim pid {} is not running", claim.pid),
            });
        };

        if let (Some(expected), Some(actual)) =
            (claim.process_group_id, observation.process_group_id)
            && expected != actual
        {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim process group id {expected} does not match observed {actual}"
                ),
            });
        }

        if let Some(actual) = &observation.executable_path
            && &claim.executable_path != actual
        {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim executable {} does not match observed {}",
                    claim.executable_path.display(),
                    actual.display()
                ),
            });
        }

        if let (Some(expected), Some(actual)) =
            (claim.process_start_time, observation.process_start_time)
            && expected != actual
        {
            return Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim process start time {expected} does not match observed {actual}"
                ),
            });
        }

        Ok(Self {
            pid,
            #[cfg(target_os = "linux")]
            pidfd: pidfd_for(pid)?,
        })
    }

    pub(crate) fn send(&self, signal: Signal) -> Result<bool> {
        #[cfg(target_os = "linux")]
        if let Some(pidfd) = &self.pidfd {
            return match pidfd_send_signal(pidfd, signal) {
                Ok(()) => Ok(true),
                Err(error) if error == Errno::SRCH => Ok(false),
                Err(error) => Err(std::io::Error::from(error).into()),
            };
        }

        match kill_process(self.pid, signal) {
            Ok(()) => Ok(true),
            Err(error) if error == Errno::SRCH => Ok(false),
            Err(error) => Err(std::io::Error::from(error).into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessObservation {
    process_group_id: Option<u32>,
    executable_path: Option<PathBuf>,
    process_start_time: Option<u64>,
}

impl ProcessObservation {
    fn observe(pid: Pid) -> Result<Option<Self>> {
        match test_kill_process(pid) {
            Ok(()) => {}
            Err(error) if error == Errno::SRCH => return Ok(None),
            Err(error) => return Err(std::io::Error::from(error).into()),
        }

        Ok(Some(Self {
            process_group_id: getpgid(Some(pid)).ok().map(pid_to_u32),
            executable_path: executable_path_for(pid),
            process_start_time: process_start_time_for(pid),
        }))
    }
}

pub(crate) async fn wait_until_claim_released(
    path: &DaemonClaimLockPath,
    timeout: Duration,
) -> Result<bool> {
    let deadline = std::time::Instant::now() + timeout;
    let lock = DaemonClaimLockAcquirer::new(path);

    loop {
        if !lock.is_held()? {
            return Ok(true);
        }

        let now = std::time::Instant::now();
        if now >= deadline {
            return Ok(false);
        }

        sleep(FORCE_WAIT_INTERVAL.min(deadline - now)).await;
    }
}

pub(crate) fn remove_stale_claim(path: &DaemonClaimPath) -> Result<()> {
    DaemonClaim::remove(path)
}

fn pid_to_u32(pid: Pid) -> u32 {
    pid.as_raw_pid() as u32
}

#[cfg(target_os = "linux")]
fn pidfd_for(pid: Pid) -> Result<Option<rustix::fd::OwnedFd>> {
    match pidfd_open(pid, PidfdFlags::empty()) {
        Ok(pidfd) => Ok(Some(pidfd)),
        Err(error) if error == Errno::SRCH => Err(Error::ForceShutdownRefused {
            reason: format!("daemon claim pid {} is not running", pid.as_raw_pid()),
        }),
        Err(error) => {
            debug!(
                pid = pid.as_raw_pid(),
                error = %error,
                "Failed to open pidfd for daemon process; falling back to pid signal"
            );
            Ok(None)
        }
    }
}

#[cfg(target_os = "linux")]
fn executable_path_for(pid: Pid) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{}/exe", pid.as_raw_pid())).ok()
}

#[cfg(not(target_os = "linux"))]
fn executable_path_for(_pid: Pid) -> Option<PathBuf> {
    None
}

#[cfg(target_os = "linux")]
fn process_start_time_for_current_process() -> Option<u64> {
    process_start_time_from_stat("/proc/self/stat")
}

#[cfg(not(target_os = "linux"))]
fn process_start_time_for_current_process() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn process_start_time_for(pid: Pid) -> Option<u64> {
    process_start_time_from_stat(format!("/proc/{}/stat", pid.as_raw_pid()))
}

#[cfg(not(target_os = "linux"))]
fn process_start_time_for(_pid: Pid) -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn process_start_time_from_stat(path: impl AsRef<Path>) -> Option<u64> {
    let stat = std::fs::read_to_string(path).ok()?;
    let fields = stat.rsplit_once(") ")?.1;

    fields.split_whitespace().nth(19)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        RuntimeDatabase,
        instance::RuntimeInstance,
        placement::{PlacementRoot, PlacementSpec},
    };

    use super::{DaemonClaim, DaemonClaimLockAcquirer, DaemonClaimLockAcquisition};

    #[cfg(target_os = "linux")]
    mod linux_stat {
        use super::super::process_start_time_from_stat;

        #[test]
        fn parses_process_start_time() {
            let stat =
                "1234 (synd daemon) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 123456 22";
            let tmp = tempfile::NamedTempFile::new().unwrap();
            std::fs::write(tmp.path(), stat).unwrap();

            assert_eq!(process_start_time_from_stat(tmp.path()), Some(123456));
        }
    }

    mod claim_lock {
        use super::*;

        #[test]
        fn reports_contention() {
            let tmp = tempfile::tempdir().unwrap();
            let placement = placement(tmp.path());
            let first = DaemonClaimLockAcquirer::new(placement.daemon_claim_lock_path())
                .try_acquire()
                .unwrap();
            let second = DaemonClaimLockAcquirer::new(placement.daemon_claim_lock_path())
                .try_acquire()
                .unwrap();

            assert!(matches!(first, DaemonClaimLockAcquisition::Acquired(_)));
            assert!(matches!(second, DaemonClaimLockAcquisition::AlreadyHeld));
        }
    }

    mod claim {
        use super::*;

        #[test]
        fn validates_matching_placement() {
            let tmp = tempfile::tempdir().unwrap();
            let placement = placement(tmp.path());
            let claim = DaemonClaim {
                format_version: super::super::CLAIM_FORMAT_VERSION,
                pid: 1,
                process_group_id: Some(1),
                runtime_instance_id: placement.instance().id().to_string(),
                canonical_database_path: placement
                    .instance()
                    .canonical_database_path()
                    .to_path_buf(),
                endpoint_path: placement.endpoint().path().to_path_buf(),
                executable_path: Path::new("/bin/synd").to_path_buf(),
                process_start_time: Some(42),
            };

            claim.validate_placement(&placement).unwrap();
        }

        #[test]
        fn rejects_mismatched_instance() {
            let tmp = tempfile::tempdir().unwrap();
            let placement = placement(tmp.path());
            let claim = DaemonClaim {
                format_version: super::super::CLAIM_FORMAT_VERSION,
                pid: 1,
                process_group_id: Some(1),
                runtime_instance_id: "other".to_owned(),
                canonical_database_path: placement
                    .instance()
                    .canonical_database_path()
                    .to_path_buf(),
                endpoint_path: placement.endpoint().path().to_path_buf(),
                executable_path: Path::new("/bin/synd").to_path_buf(),
                process_start_time: Some(42),
            };

            assert!(claim.validate_placement(&placement).is_err());
        }
    }

    fn placement(root: &Path) -> PlacementSpec {
        let db = root.join("synd.db");
        let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();

        PlacementSpec::from_instance(PlacementRoot::from(root.join("runtime")), instance)
    }
}
