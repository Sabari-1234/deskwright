use std::ffi::c_void;
use std::mem::size_of;

use deskwright_core::model::app::ProcessId;
use deskwright_core::{DeskwrightError, DeskwrightResult};

/// Extra slots beyond the reported count, for processes started between the two calls.
const SPARE_PROCESS_SLOTS: usize = 64;

/// IDs of every process currently running, read live from the kernel.
pub fn all_process_ids() -> DeskwrightResult<Vec<ProcessId>> {
    // SAFETY: a null buffer with size 0 asks only for the number of processes.
    let reported_count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    let capacity =
        usize::try_from(reported_count).map_err(|_| list_failed())? + SPARE_PROCESS_SLOTS;
    let mut process_ids: Vec<libc::pid_t> = vec![0; capacity];
    let buffer_bytes =
        i32::try_from(capacity * size_of::<libc::pid_t>()).map_err(|_| list_failed())?;
    // SAFETY: the buffer holds `capacity` pid_t values and `buffer_bytes` is exactly its size,
    // so the kernel cannot write past the end; it returns how many entries it filled.
    let filled_count =
        unsafe { libc::proc_listallpids(process_ids.as_mut_ptr().cast::<c_void>(), buffer_bytes) };
    let filled_count = usize::try_from(filled_count).map_err(|_| list_failed())?;
    process_ids.truncate(filled_count.min(capacity));
    Ok(process_ids
        .into_iter()
        .filter(|&process_id| process_id > 0)
        .map(ProcessId)
        .collect())
}

/// The path of a process's executable, or `None` when the process is gone or not readable.
pub fn executable_path(process_id: ProcessId) -> Option<String> {
    let mut path_bytes = vec![0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    let buffer_size = u32::try_from(path_bytes.len()).ok()?;
    // SAFETY: the buffer is `buffer_size` bytes long, so the kernel cannot write past it; it
    // returns the number of bytes written, or a non-positive value on failure.
    let written = unsafe {
        libc::proc_pidpath(
            process_id.0,
            path_bytes.as_mut_ptr().cast::<c_void>(),
            buffer_size,
        )
    };
    let written = usize::try_from(written)
        .ok()
        .filter(|&written| written > 0)?;
    path_bytes.truncate(written);
    String::from_utf8(path_bytes).ok()
}

/// True for the system XPC services that draw UI on behalf of another app, such as the Open
/// and Save panels of sandboxed apps. Their windows belong to the app that asked for them.
pub fn is_hosted_ui_service(executable_path: &str) -> bool {
    executable_path.starts_with(SYSTEM_PREFIX) && executable_path.contains(XPC_BUNDLE_MARKER)
}

const SYSTEM_PREFIX: &str = "/System/";
const XPC_BUNDLE_MARKER: &str = ".xpc/Contents/MacOS/";

fn list_failed() -> DeskwrightError {
    DeskwrightError::operation_failed(
        "list processes",
        std::io::Error::last_os_error().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_this_process_and_only_positive_ids() {
        let process_ids = all_process_ids().unwrap();
        let this_process = ProcessId(i32::try_from(std::process::id()).unwrap());

        assert!(process_ids.contains(&this_process));
        assert!(process_ids.iter().all(|process_id| process_id.0 > 0));
    }

    #[test]
    fn reads_the_executable_path_of_a_running_process() {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let child_id = ProcessId(i32::try_from(child.id()).unwrap());

        let path = executable_path(child_id);
        let _ = child.kill();
        let _ = child.wait();

        assert_eq!(path.as_deref(), Some("/bin/sleep"));
    }

    #[test]
    fn has_no_path_for_a_process_that_does_not_exist() {
        assert_eq!(executable_path(ProcessId(i32::MAX)), None);
    }

    #[test]
    fn recognizes_system_xpc_services_that_host_app_ui() {
        assert!(is_hosted_ui_service(
            "/System/Library/Frameworks/AppKit.framework/Versions/C/XPCServices/\
             com.apple.appkit.xpc.openAndSavePanelService.xpc/Contents/MacOS/\
             com.apple.appkit.xpc.openAndSavePanelService"
        ));
    }

    #[test]
    fn does_not_treat_regular_or_third_party_apps_as_hosted_ui() {
        assert!(!is_hosted_ui_service(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        ));
        assert!(!is_hosted_ui_service(
            "/Applications/Other.app/Contents/XPCServices/Helper.xpc/Contents/MacOS/Helper"
        ));
    }

    #[test]
    fn sees_a_process_started_after_an_earlier_listing() {
        let _earlier = all_process_ids().unwrap();
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let child_id = ProcessId(i32::try_from(child.id()).unwrap());

        let later = all_process_ids().unwrap();
        let _ = child.kill();
        let _ = child.wait();

        assert!(later.contains(&child_id));
    }
}
