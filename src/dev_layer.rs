use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use procfs::process::{FDTarget, Process};

use crate::model::{DeviceUsage, ProcessRef};

fn normalize_dev_target(path: &Path) -> Option<PathBuf> {
    let target = path.to_string_lossy();

    if !target.starts_with("/dev/") {
        return None;
    }

    if target == "/dev/null" {
        return None;
    }

    if target.contains("/shm/") {
        return None;
    }

    Some(path.to_path_buf())
}

/// Build a grouped map: /dev node -> set of processes using it.
pub fn collect_device_usage(processes: &[ProcessRef]) -> Vec<DeviceUsage> {
    let mut by_device: BTreeMap<PathBuf, BTreeSet<ProcessRef>> = BTreeMap::new();

    for proc_ref in processes {
        let process = match Process::new(proc_ref.pid) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let fds = match process.fd() {
            Ok(fds) => fds,
            Err(_) => continue,
        };

        for fd in fds {
            let fd = match fd {
                Ok(v) => v,
                Err(_) => continue,
            };

            let path = match &fd.target {
                FDTarget::Path(p) => p,
                _ => continue,
            };

            let Some(device_path) = normalize_dev_target(path) else {
                continue;
            };

            by_device
                .entry(device_path)
                .or_default()
                .insert(proc_ref.clone());
        }
    }

    by_device
        .into_iter()
        .map(|(device_path, processes)| DeviceUsage {
            device_path,
            processes,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize_dev_target;
    use std::path::PathBuf;

    #[test]
    fn accepts_regular_dev_nodes() {
        let path = PathBuf::from("/dev/video0");
        assert_eq!(normalize_dev_target(&path), Some(path));
    }

    #[test]
    fn rejects_non_dev_paths() {
        let path = PathBuf::from("/tmp/foo");
        assert_eq!(normalize_dev_target(&path), None);
    }

    #[test]
    fn rejects_dev_null() {
        let path = PathBuf::from("/dev/null");
        assert_eq!(normalize_dev_target(&path), None);
    }

    #[test]
    fn rejects_shm_paths() {
        let path = PathBuf::from("/dev/shm/test");
        assert_eq!(normalize_dev_target(&path), None);
    }
}
