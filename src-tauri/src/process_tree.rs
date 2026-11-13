//! The processes a CLI started, wherever they've gone since.
//!
//! Claude Code runs each shell command `detached`, in a session and process group of its own, so
//! signalling the CLI's group misses them: a dev server or test run an agent started would
//! outlive its thread. They're found by walking the CLI's descendants instead, and each is
//! signalled along with the group it leads.

use std::collections::{HashMap, HashSet};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// A process, told apart from a later one that reuses its pid by when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Member {
    pid: u32,
    started: u64,
}

/// Everything below some roots, not the roots themselves.
#[derive(Debug, Default)]
pub struct Family {
    members: HashSet<Member>,
}

impl Family {
    /// Taken before the roots are signalled: once a parent exits, its children are handed to
    /// launchd and the trail to them is lost.
    pub fn of(roots: &[u32]) -> Self {
        let mut family = Self::default();
        if !roots.is_empty() {
            family.add_descendants(&snapshot(), roots);
        }
        family
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Signals every member still alive, and the group each leads. Children a member started
    /// since the family was taken join it first, so a shell that forked again goes too.
    pub fn signal(&mut self, signal: i32) {
        if self.members.is_empty() {
            return;
        }
        let system = snapshot();
        let alive: Vec<u32> = self
            .members
            .iter()
            .filter(|member| {
                system
                    .process(Pid::from_u32(member.pid))
                    .is_some_and(|process| process.start_time() == member.started)
            })
            .map(|member| member.pid)
            .collect();
        self.add_descendants(&system, &alive);
        for member in &self.members {
            match system.process(Pid::from_u32(member.pid)) {
                Some(process) if process.start_time() == member.started => {
                    signal_member(member.pid, signal, true);
                }
                // The pid is someone else's now.
                Some(_) => {}
                None => signal_member(member.pid, signal, false),
            }
        }
    }

    fn add_descendants(&mut self, system: &System, roots: &[u32]) {
        let mut children = HashMap::<u32, Vec<u32>>::new();
        for (pid, process) in system.processes() {
            if let Some(parent) = process.parent() {
                children
                    .entry(parent.as_u32())
                    .or_default()
                    .push(pid.as_u32());
            }
        }
        let mut seen: HashSet<u32> = roots.iter().copied().collect();
        let mut stack = roots.to_vec();
        while let Some(pid) = stack.pop() {
            for &child in children.get(&pid).into_iter().flatten() {
                if !seen.insert(child) {
                    continue;
                }
                if let Some(process) = system.process(Pid::from_u32(child)) {
                    self.members.insert(Member {
                        pid: child,
                        started: process.start_time(),
                    });
                }
                stack.push(child);
            }
        }
    }
}

fn snapshot() -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    system
}

/// Signals the group `pid` leads, and the process itself while it's `alive`. A member that has
/// exited is still signalled by group: what it left running there keeps the group id reserved, so
/// `killpg` can only reach those. Its pid may be someone else's by now, so it's left alone.
fn signal_member(pid: u32, signal: i32, alive: bool) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    // 0 and 1 would signal our own group or every process.
    if pid <= 1 {
        return;
    }
    // SAFETY: killpg and kill take plain integers and have no memory-safety preconditions.
    unsafe {
        libc::killpg(pid, signal);
        if alive {
            libc::kill(pid, signal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    fn alive(pid: u32) -> bool {
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
            ProcessRefreshKind::nothing(),
        );
        // A killed child of an exited parent is reaped by init; until then it's a zombie.
        system
            .process(Pid::from_u32(pid))
            .is_some_and(|process| process.status() != sysinfo::ProcessStatus::Zombie)
    }

    fn wait_gone(pid: u32) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !alive(pid) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Bug: a dev server an agent started outlived its deleted thread, because the CLI starts
    /// each command in a session of its own, out of reach of a signal to the CLI's group.
    #[test]
    fn a_detached_grandchild_goes_with_the_family() {
        use std::os::unix::process::CommandExt;
        let mut parent = Command::new("python3")
            .args([
                "-c",
                "import subprocess, sys, time\n\
                 child = subprocess.Popen(['sleep', '60'], start_new_session=True)\n\
                 print(child.pid, flush=True)\n\
                 time.sleep(60)",
            ])
            .process_group(0)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(parent.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let detached: u32 = line.trim().parse().unwrap();

        let mut family = Family::of(&[parent.id()]);
        // SAFETY: killpg takes plain integers and has no memory-safety preconditions.
        unsafe { libc::killpg(parent.id() as i32, libc::SIGKILL) };
        let _ = parent.wait();
        assert!(
            alive(detached),
            "out of the parent's group, so it survives that"
        );

        family.signal(libc::SIGKILL);
        assert!(wait_gone(detached), "the family signal reaches it");
    }

    #[test]
    fn a_process_with_no_children_has_no_family() {
        let mut lone = Command::new("sleep").arg("60").spawn().unwrap();
        assert!(Family::of(&[lone.id()]).is_empty());
        let _ = lone.kill();
        let _ = lone.wait();
    }
}
