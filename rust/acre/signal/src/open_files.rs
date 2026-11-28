//! Which PBOs the game has open: that's how the terrain's WRP and the antenna files are found
//! without knowing the game's or the mods' install paths.

use std::path::PathBuf;

/// PBO files this process has open.
#[must_use]
pub fn open_pbos() -> Vec<PathBuf> {
    let mut pbos = imp::open_files()
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pbo")))
        .collect::<Vec<_>>();
    pbos.sort();
    pbos.dedup();
    pbos
}

#[cfg(target_os = "linux")]
mod imp {
    use std::path::PathBuf;

    pub fn open_files() -> Vec<PathBuf> {
        let Ok(dir) = std::fs::read_dir("/proc/self/fd") else {
            return Vec::new();
        };
        dir.filter_map(Result::ok)
            .filter_map(|e| std::fs::read_link(e.path()).ok())
            .collect()
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    //! Lists this process's handles with `NtQuerySystemInformation`
    //! (`SystemExtendedHandleInformation`), keeps disk files, and asks Windows for their paths.
    //! The handles are our own, so they're used as they are (no duplication, and no
    //! `NtQueryObject`, which can hang on some handle types).

    use std::{ffi::c_void, os::windows::ffi::OsStringExt, path::PathBuf};

    use windows_sys::{
        Wdk::System::SystemInformation::{NtQuerySystemInformation, SYSTEM_INFORMATION_CLASS},
        Win32::{
            Foundation::HANDLE,
            Storage::FileSystem::{
                FILE_NAME_NORMALIZED, FILE_TYPE_DISK, GetFileType, GetFinalPathNameByHandleW,
            },
            System::Threading::GetCurrentProcessId,
        },
    };

    const SYSTEM_EXTENDED_HANDLE_INFORMATION: SYSTEM_INFORMATION_CLASS = 64;
    const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;

    #[repr(C)]
    struct HandleEntry {
        object: *mut c_void,
        process_id: usize,
        handle: usize,
        granted_access: u32,
        creator_back_trace_index: u16,
        object_type_index: u16,
        handle_attributes: u32,
        reserved: u32,
    }

    #[repr(C)]
    struct HandleInformation {
        count: usize,
        reserved: usize,
        // followed by `count` HandleEntry
    }

    /// Access masks the C++ skipped because querying such handles can hang.
    const RISKY_ACCESS: u32 = 0x0012_019f;

    pub fn open_files() -> Vec<PathBuf> {
        let mut size: usize = 1 << 20;
        let buffer = loop {
            let mut buffer = vec![0u8; size];
            let mut needed = 0u32;
            // SAFETY: the buffer is `size` bytes and writable.
            let status = unsafe {
                NtQuerySystemInformation(
                    SYSTEM_EXTENDED_HANDLE_INFORMATION,
                    buffer.as_mut_ptr().cast(),
                    u32::try_from(size).unwrap_or(u32::MAX),
                    &raw mut needed,
                )
            };
            if status == STATUS_INFO_LENGTH_MISMATCH && size < (1 << 30) {
                size = (needed as usize).max(size * 2);
                continue;
            }
            if status < 0 {
                tracing::warn!(status, "listing open files failed");
                return Vec::new();
            }
            break buffer;
        };

        let header = size_of::<HandleInformation>();
        // SAFETY: the buffer starts with the header written by the call above; it may not be
        // aligned, so it's read unaligned.
        let count = unsafe {
            buffer
                .as_ptr()
                .cast::<HandleInformation>()
                .read_unaligned()
                .count
        };
        let entry_size = size_of::<HandleEntry>();
        let count = count.min((buffer.len() - header) / entry_size);
        // SAFETY: trivially safe.
        let pid = unsafe { GetCurrentProcessId() } as usize;

        let mut files = Vec::new();
        for i in 0..count {
            // SAFETY: within the buffer (bounded above); read unaligned to be safe.
            let entry: HandleEntry = unsafe {
                buffer
                    .as_ptr()
                    .add(header + i * entry_size)
                    .cast::<HandleEntry>()
                    .read_unaligned()
            };
            if entry.process_id != pid || entry.granted_access == RISKY_ACCESS {
                continue;
            }
            let handle = entry.handle as HANDLE;
            // SAFETY: a handle value from our own process; GetFileType fails harmlessly on
            // anything that isn't a file.
            if unsafe { GetFileType(handle) } != FILE_TYPE_DISK {
                continue;
            }
            let mut path = vec![0u16; 1024];
            // SAFETY: `path` has room for 1024 UTF-16 units.
            let len = unsafe {
                GetFinalPathNameByHandleW(handle, path.as_mut_ptr(), 1024, FILE_NAME_NORMALIZED)
            } as usize;
            if len == 0 || len >= path.len() {
                continue;
            }
            let path = std::ffi::OsString::from_wide(&path[..len]);
            let path = PathBuf::from(path);
            // `\\?\C:\…` → `C:\…`
            let path = path
                .to_str()
                .and_then(|s| s.strip_prefix(r"\\?\"))
                .map_or(path.clone(), PathBuf::from);
            files.push(path);
        }
        files
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod imp {
    pub fn open_files() -> Vec<std::path::PathBuf> {
        Vec::new()
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn finds_an_open_pbo() {
        let dir = std::env::temp_dir().join(format!("acre2_signal_open_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("held.pbo");
        std::fs::write(&path, b"x").unwrap();
        let _held = std::fs::File::open(&path).unwrap();
        let open = super::open_pbos();
        assert!(open.contains(&path), "{open:?}");
        std::fs::remove_dir_all(dir).ok();
    }
}
