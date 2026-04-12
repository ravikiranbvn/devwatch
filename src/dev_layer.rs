use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use procfs::process::{FDTarget, Process};

use crate::model::{DeviceUsage, ProcessRef};

fn normalize_dev_target(path: &PathBuf) -> Option<PathBuf> {
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

    Some(path.clone())
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