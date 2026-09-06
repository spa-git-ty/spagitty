// SPDX-License-Identifier: GPL-3.0-or-later

//! A process tree owned by one farm run. Windows starts suspended so no child
//! can escape before job assignment; Unix assigns a process group at spawn.

use std::process::{Child, Command};

#[cfg(unix)]
#[derive(Debug)]
pub struct ProcessTree(u32);

#[cfg(unix)]
impl ProcessTree {
    pub fn prepare(command: &mut Command) {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    pub fn attach(child: &Child) -> std::io::Result<Self> {
        Ok(Self(child.id()))
    }

    pub fn terminate(&self) {
        // SAFETY: the nonzero PID leads the group created only for this run.
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    use windows_sys::Win32::System::JobObjects::*;
    use windows_sys::Win32::System::Threading::*;

    #[derive(Debug)]
    pub struct ProcessTree(OwnedHandle);

    fn owned(handle: HANDLE) -> std::io::Result<OwnedHandle> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            Err(std::io::Error::last_os_error())
        } else {
            // SAFETY: the successful Win32 call transfers this handle to us.
            Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
        }
    }

    impl ProcessTree {
        pub fn prepare(command: &mut Command) {
            command.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        }

        pub fn attach(child: &Child) -> std::io::Result<Self> {
            // SAFETY: pointers reference initialized structures for the duration
            // of each call; owned handles are closed on every failure path.
            unsafe {
                let job = owned(CreateJobObjectW(std::ptr::null(), std::ptr::null()))?;
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    job.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as _,
                    std::mem::size_of_val(&limits) as u32,
                ) == 0
                    || AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) == 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                resume(child.id())?;
                Ok(Self(job))
            }
        }

        pub fn terminate(&self) {
            // SAFETY: this job contains only this run and its descendants.
            unsafe {
                TerminateJobObject(self.0.as_raw_handle(), 1);
            }
        }
    }

    fn resume(pid: u32) -> std::io::Result<()> {
        // std::process::Child exposes the process handle but not the primary
        // thread handle. A suspended new process has one thread; find it by PID.
        unsafe {
            let snapshot = owned(CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0))?;
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of_val(&entry) as u32;
            let mut more = Thread32First(snapshot.as_raw_handle(), &mut entry);
            while more != 0 {
                if entry.th32OwnerProcessID == pid {
                    let thread = owned(OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID))?;
                    if ResumeThread(thread.as_raw_handle()) == u32::MAX {
                        return Err(std::io::Error::last_os_error());
                    }
                    return Ok(());
                }
                more = Thread32Next(snapshot.as_raw_handle(), &mut entry);
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "agent's suspended thread was not found",
            ))
        }
    }
}

#[cfg(windows)]
pub use windows::ProcessTree;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{Duration, Instant};

    fn helper(mode: &str, directory: &Path) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "execution::tree::tests::tree_fixture",
                "--nocapture",
            ])
            .env("SPAGITTY_TREE_TEST_MODE", mode)
            .env("SPAGITTY_TREE_TEST_DIR", directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command
    }

    #[test]
    fn tree_fixture() {
        let Ok(mode) = std::env::var("SPAGITTY_TREE_TEST_MODE") else {
            return;
        };
        let directory =
            std::path::PathBuf::from(std::env::var_os("SPAGITTY_TREE_TEST_DIR").unwrap());
        if mode == "parent" {
            let mut descendant = helper("descendant", &directory).spawn().unwrap();
            descendant.wait().unwrap();
        } else {
            std::fs::write(directory.join("ready"), "ready").unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if directory.join("write").exists() {
                    std::fs::write(directory.join("survived"), "bad").unwrap();
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    #[test]
    fn termination_stops_descendants_before_they_can_write() {
        let directory = tempfile::tempdir().unwrap();
        let mut command = helper("parent", directory.path());
        ProcessTree::prepare(&mut command);
        let mut child = command.spawn().unwrap();
        let tree = ProcessTree::attach(&child).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !directory.path().join("ready").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let ready = directory.path().join("ready").exists();
        tree.terminate();
        child.wait().unwrap();
        assert!(ready, "descendant did not start");
        std::fs::write(directory.path().join("write"), "go").unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert!(!directory.path().join("survived").exists());
    }
}
