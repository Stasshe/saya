use std::ffi::CStr;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};

pub struct InvocationUser {
    pub uid: u32,
    pub gid: u32,
    pub home: PathBuf,
    pub used_sudo: bool,
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
pub fn resolve_invocation_user() -> Result<InvocationUser> {
    match sudo_uid()? {
        Some(uid) => lookup_passwd_entry(uid, true),
        None => lookup_passwd_entry(current_uid(), false),
    }
}

/// Permanently drops the current process to the invocation user.
///
/// This is used before writing files under the user's home so root does not
/// follow user-controlled symlinks with root privileges. Call it only after all
/// root-only work for the current command is complete.
pub fn drop_to_user(user: &InvocationUser) -> Result<()> {
    // SAFETY: geteuid/getegid take no arguments and cannot fail.
    let euid = unsafe { libc::geteuid() };
    let egid = unsafe { libc::getegid() };
    if euid == user.uid && egid == user.gid {
        return Ok(());
    }
    if euid != 0 {
        bail!(
            "cannot switch from uid {} to uid {} without root",
            euid,
            user.uid
        );
    }

    // SAFETY: setgroups is called with size 0 and a null pointer to clear
    // supplementary groups before dropping gid/uid.
    let groups_rc = unsafe { libc::setgroups(0, std::ptr::null()) };
    if groups_rc != 0 {
        bail!("setgroups failed: {}", std::io::Error::last_os_error());
    }

    // SAFETY: setgid/setuid are called with passwd-derived ids.
    let gid_rc = unsafe { libc::setgid(user.gid) };
    if gid_rc != 0 {
        bail!(
            "setgid({}) failed: {}",
            user.gid,
            std::io::Error::last_os_error()
        );
    }
    let uid_rc = unsafe { libc::setuid(user.uid) };
    if uid_rc != 0 {
        bail!(
            "setuid({}) failed: {}",
            user.uid,
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

fn sudo_uid() -> Result<Option<u32>> {
    // SAFETY: geteuid takes no arguments and cannot fail.
    if unsafe { libc::geteuid() } != 0 {
        return Ok(None);
    }
    let Some(val) = std::env::var_os("SUDO_UID") else {
        return Ok(None);
    };
    let val = val
        .to_str()
        .ok_or_else(|| anyhow!("SUDO_UID is not valid UTF-8"))?;
    let uid = val
        .parse::<u32>()
        .with_context(|| format!("SUDO_UID is not a valid uid: {val}"))?;
    Ok(Some(uid))
}

fn current_uid() -> u32 {
    // SAFETY: getuid takes no arguments and cannot fail.
    unsafe { libc::getuid() }
}

fn lookup_passwd_entry(uid: u32, used_sudo: bool) -> Result<InvocationUser> {
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

    Ok(InvocationUser {
        uid: result.pw_uid,
        gid: result.pw_gid,
        home: PathBuf::from(home),
        used_sudo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_passwd_entry_resolves_current_uid() {
        // SAFETY: getuid takes no arguments and cannot fail.
        let uid = unsafe { libc::getuid() };
        let user = lookup_passwd_entry(uid, false).unwrap();
        assert_eq!(user.uid, uid);
        assert!(!user.home.as_os_str().is_empty());
    }
}
