use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ProcessRef {
    pub pid: i32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeviceUsage {
    pub device_path: PathBuf,
    pub processes: BTreeSet<ProcessRef>,
}

#[derive(Debug, Clone)]
pub struct SysfsInfo {
    pub sysfs_path: Option<PathBuf>,
    pub subsystem: Option<String>,
    pub dev_numbers: Option<String>,
    pub driver: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub usage: DeviceUsage,
    pub sysfs: SysfsInfo,
}