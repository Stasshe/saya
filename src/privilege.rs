use std::ffi::CStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

pub struct OriginalUser {
    pub uid: u32,
    pub gid: u32,
    pub home: PathBuf,
}

pub fn require_root() -> Result<()> {
    // SAFETY: geteuid takes no arguments and cannot fail.
    let euid = unsafe { libc::geteuid() };
    if euid != 0 {
        bail!("saya needs root; run with sudo");
    }
    Ok(())
}

/// Resolves the "real" user behind a root-privileged invocation.
///
/// Under `sudo saya ...`, sudo sets `SUDO_UID` to the invoking user; we use
/// that so config lives under the invoker's home, not `/root`. Without sudo
/// (e.g. a root login shell running `saya ...` directly), there is no
/// `SUDO_UID`, so we fall back to the current effective uid (whoami) — which
/// at that point is root itself.
pub fn resolve_original_user() -> Result<OriginalUser> {
    let uid = match std::env::var("SUDO_UID") {
        Ok(val) => val
            .parse::<u32>()
            .with_context(|| format!("SUDO_UID is not a valid uid: {val}"))?,
        // SAFETY: getuid takes no arguments and cannot fail.
        Err(_) => unsafe { libc::getuid() },
    };
    lookup_passwd_entry(uid)
}

/// `chown(2)` of `path` to `user.uid`/`user.gid`.
pub fn chown_to_user(path: &Path, user: &OriginalUser) -> Result<()> {
    let path_c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .with_context(|| format!("path contains NUL byte: {}", path.display()))?;
    // SAFETY: path_c is a valid NUL-terminated string for the duration of the call.
    let rc = unsafe { libc::chown(path_c.as_ptr(), user.uid, user.gid) };
    if rc != 0 {
        bail!(
            "chown({}, {}, {}) failed: {}",
            path.display(),
            user.uid,
            user.gid,
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

fn lookup_passwd_entry(uid: u32) -> Result<OriginalUser> {
    let mut buf = vec![0u8; 1024];
    let mut result: libc::passwd = unsafe { std::mem::zeroed() };
    let mut result_ptr: *mut libc::passwd = std::ptr::null_mut();

    loop {
        // SAFETY: buf is valid for buf.len() bytes; result is a valid
        // out-pointer; result_ptr receives either null or &mut result.
        let rc = unsafe {
            libc::getpwuid_r(
                uid,
                &mut result,
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                &mut result_ptr,
            )
        };
        if rc == 0 {
            break;
        }
        if rc == libc::ERANGE {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        bail!(
            "getpwuid_r({uid}) failed: {}",
            std::io::Error::from_raw_os_error(rc)
        );
    }

    if result_ptr.is_null() {
        return Err(anyhow!("no passwd entry for uid {uid}"));
    }

    // SAFETY: result_ptr is non-null and points at `result`, which getpwuid_r
    // populated; pw_dir is a valid NUL-terminated string for as long as buf lives.
    let home = unsafe { CStr::from_ptr(result.pw_dir) }
        .to_str()
        .context("home directory is not valid UTF-8")?
        .to_string();

    Ok(OriginalUser {
        uid: result.pw_uid,
        gid: result.pw_gid,
        home: PathBuf::from(home),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_passwd_entry_resolves_current_uid() {
        // SAFETY: getuid takes no arguments and cannot fail.
        let uid = unsafe { libc::getuid() };
        let user = lookup_passwd_entry(uid).unwrap();
        assert_eq!(user.uid, uid);
        assert!(!user.home.as_os_str().is_empty());
    }
}
