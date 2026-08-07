//! HTTP and checksum helpers for the Proton-GE step.

use std::path::PathBuf;

use crate::exec::{Cmd, CmdOutput, CommandRunner};
use crate::improvement::{ParseFailure, StepError};
use crate::steps::constants::{CURL_BIN, PROTON_GE_LATEST_URL, SHA512SUM_BIN};
use crate::steps::domain::{parse_checksum, parse_release, tarball_name, ProtonRelease};

const FETCH_FLAGS: &str = "-sfL";
const DOWNLOAD_FLAGS: &str = "-sfLo";

/// Fetches the latest release metadata from the GitHub API.
pub(super) fn fetch_release(runner: &dyn CommandRunner) -> Result<ProtonRelease, StepError> {
    let output = fetch_text(runner, PROTON_GE_LATEST_URL)?;
    parse_release(&output.stdout).ok_or_else(|| StepError::Parse {
        what: "Proton-GE release",
        path: PathBuf::from(PROTON_GE_LATEST_URL),
        source: ParseFailure::Unexpected {
            expected: "a release with x86_64 tarball and checksum assets",
            found: output.stdout_trimmed().chars().take(120).collect(),
        },
    })
}

/// Downloads and verifies the tarball, returning the local temp path.
pub(super) fn download_verified(
    runner: &dyn CommandRunner,
    release: &ProtonRelease,
) -> Result<PathBuf, StepError> {
    let tarball = tarball_name(&release.tag);
    let temp_path = std::env::temp_dir().join(&tarball);
    let temp_str = temp_path.to_string_lossy().into_owned();

    let checksum_output = fetch_text(runner, &release.checksum_url)?;
    let expected_hash =
        parse_checksum(&checksum_output.stdout, &tarball).ok_or_else(|| StepError::Parse {
            what: "sha512 hash",
            path: PathBuf::from(&release.checksum_url),
            source: ParseFailure::Unexpected {
                expected: "a sha512 hash line",
                found: String::new(),
            },
        })?;

    download_file(runner, &release.tarball_url, &temp_str)?;
    verify_checksum(runner, &temp_str, &expected_hash)?;
    Ok(temp_path)
}

fn fetch_text(runner: &dyn CommandRunner, url: &str) -> Result<CmdOutput, StepError> {
    let cmd = Cmd::user(CURL_BIN).arg(FETCH_FLAGS).arg(url);
    runner.run(&cmd).map_err(StepError::Exec)
}

fn download_file(runner: &dyn CommandRunner, url: &str, dest: &str) -> Result<(), StepError> {
    let cmd = Cmd::user(CURL_BIN).arg(DOWNLOAD_FLAGS).arg(dest).arg(url);
    runner.run(&cmd).map(|_| ()).map_err(StepError::Exec)
}

fn verify_checksum(
    runner: &dyn CommandRunner,
    file_path: &str,
    expected: &str,
) -> Result<(), StepError> {
    let cmd = Cmd::user(SHA512SUM_BIN).arg(file_path);
    let output = runner.run(&cmd).map_err(StepError::Exec)?;
    let computed = output.stdout.split_whitespace().next().unwrap_or("");
    if computed != expected {
        return Err(StepError::Command {
            command: format!("sha512sum verification of {file_path}"),
            code: 1,
            stderr: format!("expected {expected}, got {computed}"),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "proton_ge_fetch_test.rs"]
mod proton_ge_fetch_test;
