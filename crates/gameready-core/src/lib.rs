//! The improvement engine behind gameready.
//!
//! An improvement is one system change with five methods: `probe`, `plan`,
//! `apply`, `verify`, `rollback`. See [`improvement::CoreImprovement`].
//!
//! Three constraints shape the code here:
//!
//! 1. Mutations go through [`improvement::ApplyCx::mutate`], which appends the
//!    undo record to the journal and fsyncs it before running the mutation.
//! 2. The executor calls `verify` after `apply`, and rolls the step back from
//!    its own journal records if a check fails.
//! 3. This crate depends on no terminal or prompting library, so steps return
//!    data and the CLI crate renders it.
//!
//! Out of scope: kernel installs, bootloader config, GPU driver changes, and
//! kernel command line parameters.

// A test reports failure by panicking, so expect, unwrap, and panic are its
// assertion mechanism. The deny in Cargo.toml targets the paths that run on a
// user's machine, where a panic would abandon a half-applied change.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]

pub mod exec;
pub mod facts;
pub mod improvement;
pub mod infra;
pub mod journal;
pub mod pkg;
pub mod rollback;
pub mod run;
pub mod steps;
