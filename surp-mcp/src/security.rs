use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("filesystem writes are disabled by SURP_MCP_READ_ONLY")]
    ReadOnly,

    #[error("path '{0}' is outside the allowed MCP roots")]
    OutsideRoots(String),

    #[error("input path '{0}' does not exist or cannot be read: {1}")]
    ReadPath(String, std::io::Error),

    #[error("output parent for '{0}' does not exist")]
    MissingParent(String),

    #[error("output path '{0}' already exists; set allow_overwrite=true to replace it")]
    OutputExists(String),

    #[error("payload is {actual} bytes, exceeding the configured maximum of {limit} bytes")]
    PayloadTooLarge { actual: usize, limit: usize },

    #[error(
        "inline output is {actual} bytes, exceeding the configured maximum of {limit} bytes; provide output_path or raise SURP_MCP_MAX_INLINE_BYTES"
    )]
    InlineTooLarge { actual: usize, limit: usize },
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    roots: Vec<PathBuf>,
    read_only: bool,
    pub max_input_bytes: usize,
    pub max_inline_bytes: usize,
    pub max_text_bytes: usize,
    /// Maximum accepted length, in bytes, of a single raw JSON-RPC message
    /// (one stdin line) before it is even handed to the JSON parser. This
    /// bounds worst-case memory/parse cost from a single pathological
    /// request, independent of the per-argument limits above which only
    /// apply after a message has already been parsed.
    pub max_line_bytes: usize,
}

impl SecurityConfig {
    pub fn from_env() -> Result<Self, SecurityError> {
        let roots = match env::var_os("SURP_MCP_ROOTS") {
            Some(value) => env::split_paths(&value).collect::<Vec<_>>(),
            None => vec![env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
        };

        let mut canonical_roots = Vec::with_capacity(roots.len());
        for root in roots {
            let canonical = fs::canonicalize(&root)
                .map_err(|err| SecurityError::ReadPath(root.display().to_string(), err))?;
            canonical_roots.push(canonical);
        }

        Ok(Self {
            roots: canonical_roots,
            // This server executes filesystem operations on behalf of
            // potentially untrusted LLM/agent input arriving over stdio, so
            // it defaults to read-only unless an operator explicitly opts
            // into writes with SURP_MCP_READ_ONLY set to a falsy value
            // (e.g. "false"). See resources/security.md.
            read_only: env_bool("SURP_MCP_READ_ONLY", true),
            max_input_bytes: env_usize("SURP_MCP_MAX_INPUT_BYTES", 16 * 1024 * 1024),
            max_inline_bytes: env_usize("SURP_MCP_MAX_INLINE_BYTES", 4 * 1024 * 1024),
            max_text_bytes: env_usize("SURP_MCP_MAX_TEXT_BYTES", 4 * 1024 * 1024),
            // A raw RPC line can legitimately contain a base64-encoded
            // payload up to max_input_bytes (~1.34x inflation) plus JSON
            // envelope overhead, so give it generous headroom over
            // max_input_bytes while still bounding worst-case size.
            max_line_bytes: env_usize("SURP_MCP_MAX_LINE_BYTES", 32 * 1024 * 1024),
        })
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn assert_payload_size(&self, len: usize) -> Result<(), SecurityError> {
        if len > self.max_input_bytes {
            Err(SecurityError::PayloadTooLarge {
                actual: len,
                limit: self.max_input_bytes,
            })
        } else {
            Ok(())
        }
    }

    pub fn assert_text_size(&self, len: usize) -> Result<(), SecurityError> {
        if len > self.max_text_bytes {
            Err(SecurityError::PayloadTooLarge {
                actual: len,
                limit: self.max_text_bytes,
            })
        } else {
            Ok(())
        }
    }

    pub fn assert_inline_size(&self, len: usize) -> Result<(), SecurityError> {
        if len > self.max_inline_bytes {
            Err(SecurityError::InlineTooLarge {
                actual: len,
                limit: self.max_inline_bytes,
            })
        } else {
            Ok(())
        }
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, SecurityError> {
        let canonical = self.canonical_input_path(path)?;
        let data =
            fs::read(&canonical).map_err(|err| SecurityError::ReadPath(path.to_string(), err))?;
        self.assert_payload_size(data.len())?;
        Ok(data)
    }

    pub fn write_file(
        &self,
        path: &str,
        bytes: &[u8],
        allow_overwrite: bool,
    ) -> Result<PathBuf, SecurityError> {
        if self.read_only {
            return Err(SecurityError::ReadOnly);
        }
        let output = self.validate_output_path(path, allow_overwrite)?;
        fs::write(&output, bytes)
            .map_err(|err| SecurityError::ReadPath(output.display().to_string(), err))?;
        Ok(output)
    }

    fn canonical_input_path(&self, path: &str) -> Result<PathBuf, SecurityError> {
        let canonical = fs::canonicalize(Path::new(path))
            .map_err(|err| SecurityError::ReadPath(path.to_string(), err))?;
        self.ensure_in_roots(&canonical)?;
        Ok(canonical)
    }

    fn validate_output_path(
        &self,
        path: &str,
        allow_overwrite: bool,
    ) -> Result<PathBuf, SecurityError> {
        let requested = PathBuf::from(path);
        if requested.exists() && !allow_overwrite {
            return Err(SecurityError::OutputExists(path.to_string()));
        }
        let parent = requested
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent =
            fs::canonicalize(parent).map_err(|_| SecurityError::MissingParent(path.to_string()))?;
        self.ensure_in_roots(&parent)?;
        let output = parent.join(
            requested
                .file_name()
                .ok_or_else(|| SecurityError::MissingParent(path.to_string()))?,
        );

        // The parent-only check above is not sufficient by itself: if a
        // file (or, critically, a symlink) already exists at `output` — for
        // example planted inside an allowed root by an earlier, less
        // trusted operation — `fs::write` will follow that symlink at write
        // time. That can escape the sandbox even though the parent
        // directory itself is inside `roots`, because the final path
        // component was never resolved or re-checked. So when something
        // already exists at `output`, canonicalize the *entire* path
        // (resolving the final component too) and re-validate it against
        // `roots`. A path that doesn't exist yet can't be canonicalized
        // (`fs::canonicalize` errors on missing paths), so legitimate new
        // file writes keep relying on the parent-only check above.
        if output.exists() {
            let fully_resolved = fs::canonicalize(&output)
                .map_err(|err| SecurityError::ReadPath(output.display().to_string(), err))?;
            self.ensure_in_roots(&fully_resolved)?;
        }

        Ok(output)
    }

    fn ensure_in_roots(&self, path: &Path) -> Result<(), SecurityError> {
        if self.roots.iter().any(|root| path.starts_with(root)) {
            Ok(())
        } else {
            Err(SecurityError::OutsideRoots(path.display().to_string()))
        }
    }
}

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    /// Serializes tests in this module that mutate process-wide env vars
    /// (`SURP_MCP_READ_ONLY`), since `std::env::set_var`/`remove_var` are
    /// not safe to call concurrently from multiple threads.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn unique_temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!(
            "surp-mcp-security-test-{label}-{}-{}-{}",
            std::process::id(),
            nanos,
            n
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        // Canonicalize so comparisons against `roots` (which are always
        // stored canonicalized) behave the same as in production, since
        // platforms such as macOS route /tmp through a symlink.
        fs::canonicalize(&dir).expect("canonicalize temp dir")
    }

    fn config_with_root(root: PathBuf, read_only: bool) -> SecurityConfig {
        SecurityConfig {
            roots: vec![root],
            read_only,
            max_input_bytes: 16 * 1024 * 1024,
            max_inline_bytes: 4 * 1024 * 1024,
            max_text_bytes: 4 * 1024 * 1024,
            max_line_bytes: 32 * 1024 * 1024,
        }
    }

    #[test]
    fn validate_output_path_rejects_symlink_escaping_root() {
        let root = unique_temp_dir("escape-root");
        let outside = unique_temp_dir("escape-outside");

        let link_path = root.join("escape.surp");
        let target_path = outside.join("secret.surp");
        fs::write(&target_path, b"outside contents").expect("write target file");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_path, &link_path).expect("create symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target_path, &link_path).expect("create symlink");

        let config = config_with_root(root.clone(), false);

        // Prior to the fix, this would succeed: `validate_output_path` only
        // canonicalized and checked the *parent* directory (which is inside
        // `root`), then joined the raw, unresolved file name onto it,
        // letting the symlink escape the sandbox at write time.
        let result = config.validate_output_path(link_path.to_str().unwrap(), true);

        assert!(
            matches!(result, Err(SecurityError::OutsideRoots(_))),
            "expected symlink escaping the root to be rejected, got {result:?}"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn validate_output_path_allows_new_file_inside_root() {
        let root = unique_temp_dir("newfile-root");
        let target = root.join("new-output.surp");

        let config = config_with_root(root.clone(), false);
        let result = config
            .validate_output_path(target.to_str().unwrap(), false)
            .expect("new file inside root must be allowed");
        assert_eq!(result, target);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_output_path_still_allows_overwriting_regular_file_in_root() {
        let root = unique_temp_dir("overwrite-root");
        let target = root.join("existing.surp");
        fs::write(&target, b"old contents").expect("write existing file");

        let config = config_with_root(root.clone(), false);
        let result = config
            .validate_output_path(target.to_str().unwrap(), true)
            .expect("overwriting a plain file inside root must still be allowed");
        assert_eq!(result, target);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn read_only_defaults_to_true_and_respects_explicit_opt_out() {
        let _guard = ENV_GUARD
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let previous = env::var("SURP_MCP_READ_ONLY").ok();

        // SAFETY: serialized by ENV_GUARD; no other test in this binary
        // mutates SURP_MCP_READ_ONLY.
        unsafe {
            env::remove_var("SURP_MCP_READ_ONLY");
        }
        let unset_config = SecurityConfig::from_env().expect("from_env with default roots");
        assert!(
            unset_config.read_only,
            "server must default to read-only when SURP_MCP_READ_ONLY is unset"
        );

        // SAFETY: see above.
        unsafe {
            env::set_var("SURP_MCP_READ_ONLY", "false");
        }
        let opt_out_config = SecurityConfig::from_env().expect("from_env with explicit opt-out");
        assert!(
            !opt_out_config.read_only,
            "explicit SURP_MCP_READ_ONLY=false must disable read-only mode"
        );

        // SAFETY: see above.
        unsafe {
            match &previous {
                Some(value) => env::set_var("SURP_MCP_READ_ONLY", value),
                None => env::remove_var("SURP_MCP_READ_ONLY"),
            }
        }
    }
}
