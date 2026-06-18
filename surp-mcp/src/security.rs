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
            read_only: env_bool("SURP_MCP_READ_ONLY", false),
            max_input_bytes: env_usize("SURP_MCP_MAX_INPUT_BYTES", 16 * 1024 * 1024),
            max_inline_bytes: env_usize("SURP_MCP_MAX_INLINE_BYTES", 4 * 1024 * 1024),
            max_text_bytes: env_usize("SURP_MCP_MAX_TEXT_BYTES", 4 * 1024 * 1024),
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
        Ok(parent.join(
            requested
                .file_name()
                .ok_or_else(|| SecurityError::MissingParent(path.to_string()))?,
        ))
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
