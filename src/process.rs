//! Child processes for CLI-backed providers and scorers: PATHEXT-aware
//! resolution, no console window, stdin input, and a tree kill on timeout.

use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Find `name` the way a shell would: an explicit path as is, otherwise each
/// PATH entry, trying PATHEXT suffixes on Windows (`opencode` -> `opencode.cmd`).
pub fn resolve_bin(
    name: &str,
    path: Option<&std::ffi::OsStr>,
    pathext: Option<&str>,
) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.components().count() > 1 || direct.is_absolute() {
        return direct.is_file().then(|| direct.to_path_buf());
    }
    let exts: Vec<String> = if cfg!(windows) {
        pathext
            .unwrap_or(".COM;.EXE;.BAT;.CMD")
            .split(';')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_ascii_lowercase())
            .collect()
    } else {
        Vec::new()
    };
    let has_ext = Path::new(name).extension().is_some();
    for dir in std::env::split_paths(path?) {
        // On Windows a bare `opencode` is the POSIX shim, not runnable.
        if !cfg!(windows) || has_ext {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        for ext in &exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// [`resolve_bin`] against this process's PATH and PATHEXT.
pub fn resolve_on_path(name: &str) -> Option<PathBuf> {
    resolve_bin(
        name,
        std::env::var_os("PATH").as_deref(),
        std::env::var("PATHEXT").ok().as_deref(),
    )
}

/// A command with piped stdio and no console window.
pub fn command(bin: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(bin);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Windows uses TreeKill instead: killing the parent first would
        // orphan the grandchild before taskkill could walk the tree.
        .kill_on_drop(cfg!(not(windows)));
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[derive(Debug)]
pub enum RunError {
    Spawn(std::io::Error),
    Wait(std::io::Error),
    Timeout,
}

/// Spawn `cmd`, write `input` to its stdin, and wait up to `timeout_ms`.
/// On timeout or cancellation the whole process tree is killed.
pub async fn run_with_input(
    mut cmd: tokio::process::Command,
    input: String,
    timeout_ms: u64,
) -> std::result::Result<Output, RunError> {
    let mut child = cmd.spawn().map_err(RunError::Spawn)?;
    // A .cmd shim runs the real binary as a grandchild.
    let mut guard = TreeKill(child.id());
    if let Some(mut stdin) = child.stdin.take() {
        tokio::spawn(async move {
            let _ = stdin.write_all(input.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }
    let output =
        match tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait_with_output())
            .await
        {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(RunError::Wait(e)),
            Err(_) => return Err(RunError::Timeout),
        };
    guard.0 = None;
    Ok(output)
}

#[cfg_attr(not(windows), allow(dead_code))]
struct TreeKill(Option<u32>);

impl Drop for TreeKill {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(pid) = self.0 {
            let _ = std::process::Command::new("taskkill")
                .args(["/T", "/F", "/PID", &pid.to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_honours_pathext() {
        let dir = tempfile::tempdir().unwrap();
        let shim = if cfg!(windows) { "tool.cmd" } else { "tool" };
        std::fs::write(dir.path().join(shim), "").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        let found = resolve_bin("tool", Some(&path), Some(".EXE;.CMD")).unwrap();
        assert_eq!(found, dir.path().join(shim));
        assert!(resolve_bin("missing", Some(&path), Some(".EXE;.CMD")).is_none());
        let explicit = dir.path().join(shim);
        assert_eq!(
            resolve_bin(explicit.to_str().unwrap(), None, None),
            Some(explicit.clone())
        );
    }
}
