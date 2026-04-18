use procfs::process::{FDTarget, Process};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

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

fn should_skip_dev_entry(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(v) => v,
        None => return true,
    };

    matches!(
        name,
        "fd" | "stdin"
            | "stdout"
            | "stderr"
            | "core"
            | "shm"
            | "pts"
            | "mqueue"
            | "hugepages"
            | "bus"
            | "char"
            | "block"
            | "disk"
            | "mapper"
            | "dri"
            | "snd"
            | "input"
            | "v4l"
            | "net"
            | "vfio"
    )
}

pub fn list_all_device_nodes() -> Vec<PathBuf> {
    let mut devices = Vec::new();

    let entries = match fs::read_dir("/dev") {
        Ok(v) => v,
        Err(_) => return devices,
    };

    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };

        let path = entry.path();

        if should_skip_dev_entry(&path) {
            continue;
        }

        let metadata = match fs::symlink_metadata(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let file_type = metadata.file_type();

        if file_type.is_dir() {
            continue;
        }

        let Some(device_path) = normalize_dev_target(&path) else {
            continue;
        };

        devices.push(device_path);
    }

    devices.sort();
    devices.dedup();
    devices
}

pub fn collect_all_devices_with_usage(processes: &[ProcessRef]) -> Vec<DeviceUsage> {
    let usage_records = collect_device_usage(processes);

    let mut by_device: BTreeMap<PathBuf, BTreeSet<ProcessRef>> = BTreeMap::new();

    for usage in usage_records {
        by_device.insert(usage.device_path, usage.processes);
    }

    for device_path in list_all_device_nodes() {
        by_device.entry(device_path).or_default();
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
