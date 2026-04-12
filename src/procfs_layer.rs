use crate::model::ProcessRef;
use procfs::process::all_processes;

/// Return a stable list of processes that we can inspect.
pub fn list_processes() -> Vec<ProcessRef> {
    let mut result = Vec::new();

    let processes = match all_processes() {
        Ok(p) => p,
        Err(_) => return result,
    };

    for proc_entry in processes {
        let process = match proc_entry {
            Ok(p) => p,
            Err(_) => continue,
        };

        let pid = process.pid();
        let name = match process.stat() {
            Ok(stat) => stat.comm,
            Err(_) => format!("pid-{pid}"),
        };

        result.push(ProcessRef { pid, name });
    }

    result.sort_by_key(|p| p.pid);
    result
}