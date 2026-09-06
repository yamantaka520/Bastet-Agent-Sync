//! A bounded scheduler. Storage families and overlapping source paths never run concurrently.
use super::Worker;
use crate::{model::Agent, sync::bundle::Result};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

pub const MAX_PARALLEL: usize = 3;
#[derive(Debug)]
pub struct Task {
    pub canonical: String,
    pub path: Option<PathBuf>,
    pub indices: Vec<usize>,
}
fn normalized(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    std::fs::canonicalize(&path).unwrap_or(path)
}
pub fn plan(selected: &[String], agents: &[Agent]) -> (Vec<Task>, Vec<Vec<usize>>) {
    let mut tasks: Vec<Task> = Vec::new();
    for (index, agent) in selected.iter().enumerate() {
        let canonical = match agent.as_str() {
            "claude" => "claude-code",
            "chatgpt-work" => "codex",
            a => a,
        };
        let source = agents.iter().find(|a| &a.id == agent);
        let source = if agent == "claude" {
            agents.iter().find(|a| a.id == "claude-code").or(source)
        } else {
            source
        };
        let path = source.map(|a| normalized(&a.path));
        if let Some(task) = tasks
            .iter_mut()
            .find(|t| t.canonical == canonical && t.path == path)
        {
            task.indices.push(index);
        } else {
            tasks.push(Task {
                canonical: canonical.into(),
                path,
                indices: vec![index],
            });
        }
    }
    let overlaps = |a: &Task, b: &Task| {
        a.canonical == b.canonical
            || match (&a.path, &b.path) {
                (Some(a), Some(b)) => a.starts_with(b) || b.starts_with(a),
                _ => false,
            }
    };
    // Merge connected components, including transitive path overlaps.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in 0..tasks.len() {
        let mut group = vec![i];
        let mut j = 0;
        while j < groups.len() {
            if groups[j]
                .iter()
                .any(|&other| overlaps(&tasks[i], &tasks[other]))
            {
                group.extend(groups.remove(j));
            } else {
                j += 1;
            }
        }
        group.sort_unstable();
        groups.push(group);
    }
    groups.sort_by_key(|g| g[0]);
    (tasks, groups)
}
/// Pause and claiming a group serialize on the same control lock. All spawned threads
/// are joined before returning, so a new cycle can never overlap the previous cycle.
pub fn run<T: Sync>(groups: &[T], worker: &Worker, action: impl Fn(&T) + Sync) -> Result<()> {
    let next = AtomicUsize::new(0);
    let failed = Mutex::new(false);
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..MAX_PARALLEL.min(groups.len()) {
            let action = &action;
            let next = &next;
            let failed = &failed;
            let thread = std::thread::Builder::new()
                .name("bastet-source".into())
                .spawn_scoped(scope, move || loop {
                    let index = {
                        let control = match worker.0 .0.lock() {
                            Ok(c) => c,
                            Err(_) => break,
                        };
                        if control.stop || *failed.lock().unwrap_or_else(|e| e.into_inner()) {
                            break;
                        }
                        next.fetch_add(1, Ordering::Relaxed)
                    };
                    let Some(group) = groups.get(index) else {
                        break;
                    };
                    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action(group)))
                        .is_err()
                    {
                        *failed.lock().unwrap_or_else(|e| e.into_inner()) = true;
                        break;
                    }
                });
            match thread {
                Ok(handle) => handles.push(handle),
                Err(_) => {
                    *failed.lock().unwrap_or_else(|e| e.into_inner()) = true;
                    break;
                }
            }
        }
        for handle in handles {
            if handle.join().is_err() {
                *failed.lock().unwrap_or_else(|e| e.into_inner()) = true;
            }
        }
    });
    if *failed.lock().unwrap_or_else(|e| e.into_inner()) {
        Err("sync_worker_failed".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Condvar};
    use std::time::Duration;
    fn agent(id: &str, path: &str) -> Agent {
        Agent {
            id: id.into(),
            path: path.into(),
            detected: true,
            custom: false,
        }
    }
    #[test]
    fn aliases_deduplicate_and_shared_journals_or_overlapping_paths_serialize() {
        let selected = [
            "claude",
            "claude-code",
            "codex",
            "chatgpt-work",
            "pi",
            "grok",
            "agent-memory-os",
        ]
        .map(str::to_string);
        let agents = vec![
            agent("claude", "/desktop"),
            agent("claude-code", "/claude"),
            agent("codex", "/codex"),
            agent("chatgpt-work", "/codex"),
            agent("pi", "/shared/pi"),
            agent("grok", "/shared"),
            agent("agent-memory-os", "/memory"),
        ];
        let (tasks, groups) = plan(&selected, &agents);
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].indices, [0, 1]);
        assert_eq!(tasks[1].indices, [2, 3]);
        assert_eq!(groups, [vec![0], vec![1], vec![2, 3], vec![4]]);
        let agents = vec![agent("codex", "/one"), agent("chatgpt-work", "/two")];
        let (tasks, groups) = plan(&["codex".into(), "chatgpt-work".into()], &agents);
        assert_eq!(tasks.len(), 2);
        assert_eq!(groups, [vec![0, 1]]); // Both use the canonical Codex journal.
    }
    fn exercise(pause: bool) {
        let worker = Worker::default();
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let completed = AtomicUsize::new(0);
        let gate = (Mutex::new(false), Condvar::new());
        let (tx, rx) = mpsc::channel();
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                run(&[0, 1, 2, 3, 4, 5, 6], &worker, |_| {
                    let n = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(n, Ordering::SeqCst);
                    tx.send(()).unwrap();
                    let released = gate.0.lock().unwrap();
                    let _wait = gate
                        .1
                        .wait_timeout_while(released, Duration::from_secs(5), |r| !*r)
                        .unwrap();
                    active.fetch_sub(1, Ordering::SeqCst);
                    completed.fetch_add(1, Ordering::SeqCst);
                })
            });
            let started = (0..3)
                .map(|_| rx.recv_timeout(Duration::from_secs(3)).is_ok())
                .collect::<Vec<_>>();
            if pause {
                worker.0 .0.lock().unwrap().stop = true;
            }
            let excess = rx.try_recv().is_ok();
            *gate.0.lock().unwrap() = true;
            gate.1.notify_all();
            handle.join().unwrap().unwrap();
            assert!(started.into_iter().all(|v| v), "three groups must overlap");
            assert!(!excess);
        });
        assert_eq!(peak.load(Ordering::SeqCst), MAX_PARALLEL);
        assert_eq!(active.load(Ordering::SeqCst), 0); // All active jobs joined before return.
        assert_eq!(completed.load(Ordering::SeqCst), if pause { 3 } else { 7 });
    }
    #[test]
    fn exactly_three_groups_overlap_and_all_work_completes() {
        exercise(false);
    }
    #[test]
    fn pause_stops_dispatch_and_waits_for_active_groups() {
        exercise(true);
    }
    #[test]
    fn panic_is_reported_after_threads_join_instead_of_leaving_worker_running() {
        let result = run(&[0], &Worker::default(), |_| panic!("fixture"));
        assert_eq!(result.unwrap_err(), "sync_worker_failed");
    }
}
