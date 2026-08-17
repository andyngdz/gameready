//! Rollback planning and preview data.

mod service;

pub(crate) use service::{preview_rows, rollback_plan, PreviewRow};
