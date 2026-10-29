//! Optional native process memory limit selected per Atlas instance.
//! This uses a Windows job, not a JVM heap flag or a polling side process.

pub fn megabytes(value: Option<&str>) -> Result<Option<usize>, String> {
    match value {
        None | Some("" | "0") => Ok(None),
        Some(value) => value
            .parse::<usize>()
            .ok()
            .filter(|n| (2048..=1_048_576).contains(n) && n % 256 == 0)
            .map(Some)
            .ok_or_else(|| "Invalid Atlas memory limit".to_owned()),
    }
}

#[cfg(windows)]
mod platform {
    use std::{ffi::c_void, mem::size_of};
    type Handle = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimits {
        process_time: i64,
        job_time: i64,
        flags: u32,
        min_working_set: usize,
        max_working_set: usize,
        active_processes: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimits {
        basic: BasicLimits,
        io_counters: [u64; 6],
        process_memory: usize,
        job_memory: usize,
        peak_process_memory: usize,
        peak_job_memory: usize,
    }
    // JOBOBJECT_BASIC_LIMIT_INFORMATION / JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    // from winnt.h. An incorrect layout can silently set a different limit.
    #[cfg(target_pointer_width = "64")]
    const _: () = {
        assert!(size_of::<BasicLimits>() == 64);
        assert!(size_of::<ExtendedLimits>() == 144);
        assert!(std::mem::offset_of!(ExtendedLimits, process_memory) == 112);
    };
    #[cfg(target_pointer_width = "32")]
    const _: () = {
        assert!(size_of::<BasicLimits>() == 48);
        assert!(size_of::<ExtendedLimits>() == 112);
        assert!(std::mem::offset_of!(ExtendedLimits, process_memory) == 96);
    };
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(
            job: Handle,
            class: i32,
            data: *const c_void,
            length: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn GetCurrentProcess() -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        #[cfg(test)]
        fn QueryInformationJobObject(
            job: Handle,
            class: i32,
            data: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
    }
    pub struct MemoryLimit(Handle);
    impl Drop for MemoryLimit {
        fn drop(&mut self) {
            // SAFETY: this handle was created by us and has a single owner.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    pub fn apply(mb: usize) -> Result<MemoryLimit, String> {
        if !(2048..=1_048_576).contains(&mb) || mb % 256 != 0 {
            return Err("Invalid Atlas memory limit".to_owned());
        }
        let bytes = mb
            .checked_mul(1024 * 1024)
            .ok_or("Atlas memory limit overflow")?;
        // SAFETY: null attributes/name request an unnamed, non-inheritable job.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let limit = MemoryLimit(job);
        let mut info = ExtendedLimits::default();
        info.basic.flags = 0x100; // JOB_OBJECT_LIMIT_PROCESS_MEMORY (committed memory).
        info.process_memory = bytes;
        // SAFETY: C-layout buffer matches JOBOBJECT_EXTENDED_LIMIT_INFORMATION;
        // it stays alive throughout the call. Class 9 is ExtendedLimitInformation.
        if unsafe {
            SetInformationJobObject(
                job,
                9,
                (&info as *const ExtendedLimits).cast(),
                size_of::<ExtendedLimits>() as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        // SAFETY: both handles refer to live kernel objects. No kill-on-close
        // flag is set, so releasing the limit never terminates the game.
        if unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(limit)
    }

    #[cfg(test)]
    pub(super) fn assert_applied(limit: &MemoryLimit, mb: usize) {
        let mut info = ExtendedLimits::default();
        // SAFETY: the queried handle is owned by a live guard, and the
        // initialized C-layout output buffer has exactly the advertised size.
        let result = unsafe {
            QueryInformationJobObject(
                limit.0,
                9,
                (&mut info as *mut ExtendedLimits).cast(),
                size_of::<ExtendedLimits>() as u32,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(result, 0, "{}", std::io::Error::last_os_error());
        assert_eq!(
            info.basic.flags, 0x100,
            "process commit limit only, no kill-on-close"
        );
        assert_eq!(info.process_memory, mb * 1024 * 1024);
        assert_eq!(
            info.job_memory, 0,
            "additional processes keep their own budgets"
        );
    }
}
#[cfg(windows)]
pub use platform::{MemoryLimit, apply};

#[cfg(not(windows))]
pub struct MemoryLimit;
#[cfg(not(windows))]
pub fn apply(_: usize) -> Result<MemoryLimit, String> {
    Err("Atlas process memory limits currently require Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_budget_is_strict_and_checked() {
        assert_eq!(megabytes(None), Ok(None));
        assert_eq!(megabytes(Some("0")), Ok(None));
        assert_eq!(megabytes(Some("4096")), Ok(Some(4096)));
        assert_eq!(megabytes(Some("2048")), Ok(Some(2048)));
        assert_eq!(megabytes(Some("1048576")), Ok(Some(1_048_576)));
        for bad in [
            "-1",
            "1",
            "2049",
            "1048832",
            "hello",
            "4096 ",
            "999999999999999999999999",
        ] {
            assert!(megabytes(Some(bad)).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_job_enforces_commit_budget_in_child_only() {
        const CHILD: &str = "ATLAS_MEMORY_REGRESSION_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let limit = apply(2048).expect("apply isolated child budget");
            platform::assert_applied(&limit, 2048);
            let mut small = Vec::<u8>::new();
            small
                .try_reserve_exact(8 * 1024 * 1024)
                .expect("normal game allocation");
            small.resize(8 * 1024 * 1024, 7);
            assert_eq!(small[small.len() - 1], 7);
            let mut over_budget = Vec::<u8>::new();
            assert!(
                over_budget
                    .try_reserve_exact(3 * 1024 * 1024 * 1024)
                    .is_err(),
                "Windows must refuse committed memory above 2 GiB"
            );
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::windows_job_enforces_commit_budget_in_child_only",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .expect("start isolated regression child");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
