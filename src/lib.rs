//! Platform-independent EPUB processing APIs.
//!
//! Callers provide all platform-owned paths through [`EngineConfig`] and
//! receive structured progress through [`TaskReporter`]. The crate deliberately
//! has no dependency on Tauri, Android, protobuf, or UI types.

mod backend;
mod encoding;
pub mod task;

use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

pub use task::{
    ChineseConversionDirection, FileIssue, FontTaskOptions, ImageTaskOptions, OcrCharPolicy,
    ReplaceCoverOptions, TaskEvent, TaskOptions, TaskResult, TaskSpec, TaskSummary, TaskType,
};

#[cfg(feature = "font")]
pub use backend::font::decrypt_font::OcrModelVerification;

/// Per-request engine configuration supplied by the embedding application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    log_path: PathBuf,
    opencc_dir: Option<PathBuf>,
    ocr_model_dir: Option<PathBuf>,
}

impl EngineConfig {
    pub fn new(log_path: impl Into<PathBuf>) -> Self {
        Self {
            log_path: log_path.into(),
            opencc_dir: None,
            ocr_model_dir: None,
        }
    }

    pub fn with_opencc_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.opencc_dir = Some(directory.into());
        self
    }

    pub fn with_ocr_model_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.ocr_model_dir = Some(directory.into());
        self
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn opencc_dir(&self) -> Option<&Path> {
        self.opencc_dir.as_deref()
    }

    pub fn ocr_model_dir(&self) -> Option<&Path> {
        self.ocr_model_dir.as_deref()
    }
}

/// Error returned by the platform-independent engine API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError {
    message: String,
}

impl EngineError {
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for EngineError {}

impl From<String> for EngineError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for EngineError {
    fn from(message: &str) -> Self {
        Self::from(message.to_string())
    }
}

/// Receives typed task events in execution order.
pub trait TaskReporter {
    fn report(&mut self, event: TaskEvent) -> Result<(), EngineError>;
}

impl<F> TaskReporter for F
where
    F: FnMut(TaskEvent) -> Result<(), EngineError>,
{
    fn report(&mut self, event: TaskEvent) -> Result<(), EngineError> {
        self(event)
    }
}

/// Returns whether the compiled engine supports the task contract.
pub fn supports(spec: &TaskSpec) -> bool {
    backend::supports(spec)
}

/// Runs one typed EPUB task through the shared engine.
pub fn run(
    spec: TaskSpec,
    config: EngineConfig,
    mut reporter: impl TaskReporter,
) -> Result<TaskResult, EngineError> {
    backend::run(&spec, &config, &mut |event| {
        reporter.report(event).map_err(|error| error.to_string())
    })
    .map_err(EngineError::from)
}

#[cfg(feature = "font")]
pub fn list_font_targets(input: &Path) -> Result<Vec<String>, EngineError> {
    backend::font::font_targets::list_font_targets(input).map_err(EngineError::from)
}

#[cfg(feature = "font")]
pub fn verify_ocr_model_dir(model_dir: &Path) -> Result<OcrModelVerification, EngineError> {
    backend::font::decrypt_font::verify_ocr_model_dir(model_dir).map_err(EngineError::from)
}
