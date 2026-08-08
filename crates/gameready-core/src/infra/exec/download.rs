//! Fetching a file over HTTPS, counting it as it lands.

use std::fs::File;
use std::io::{Read as _, Write as _};
use std::path::Path;

use crate::exec::ExecError;

/// How much is read from the socket at a time.
///
/// Also how often the caller hears about it. Small enough that a bar moves on a
/// slow line, large enough that a 178 MB transfer reports about three thousand
/// times rather than per packet.
const CHUNK: usize = 64 * 1024;

/// Downloads `url` into `dest`, calling `on_bytes` with the running total.
///
/// Streamed rather than read into memory: the one thing gameready fetches is a
/// Proton build, and holding 178 MB in a `Vec` to write it straight back out
/// would be the whole file in memory for no reason.
pub(super) fn fetch(url: &str, dest: &Path, on_bytes: &dyn Fn(u64)) -> Result<(), ExecError> {
    let failed = |detail: String| ExecError::Download {
        url: url.to_owned(),
        detail,
    };

    let response = ureq::get(url)
        .call()
        .map_err(|source| failed(source.to_string()))?;
    let mut body = response.into_body().into_reader();

    let mut file = File::create(dest).map_err(|source| ExecError::Write {
        path: dest.to_path_buf(),
        source,
    })?;

    let mut buffer = vec![0_u8; CHUNK];
    let mut done: u64 = 0;
    loop {
        let read = body
            .read(&mut buffer)
            .map_err(|source| failed(source.to_string()))?;
        if read == 0 {
            return file.flush().map_err(|source| ExecError::Write {
                path: dest.to_path_buf(),
                source,
            });
        }
        file.write_all(&buffer[..read])
            .map_err(|source| ExecError::Write {
                path: dest.to_path_buf(),
                source,
            })?;
        done = done.saturating_add(read as u64);
        on_bytes(done);
    }
}

#[cfg(test)]
#[path = "download_test.rs"]
mod download_test;
