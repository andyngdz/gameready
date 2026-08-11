# Proton-GE asset and directory names come off the release, not the tag

Case:
Case: Upstream renamed the x86_64 assets at GE-Proton11-4. Up to 11-3 the
tarball was `<tag>.tar.gz`; from 11-4 it is `<tag>-x86_64.tar.gz`, and the
directory it extracts to follows the filename, not the tag. Upstream's own
`compatibilitytool.vdf` registers the tool under that same directory name
(`GE-Proton11-5-x86_64`), so that is what Steam shows and what
`CompatToolMapping` needs.

Anything built from the tag goes stale. Two failures came out of it: the sha512
lookup found no matching line in the checksum file and the step died before it
downloaded a byte, and the install directory check looked at a path that never
exists so verify failed after a good 508 MB install.

Read `tarball_name` off the release JSON and take the install directory from it
(`ProtonRelease::install_name`, tarball name minus `.tar.gz`). Fixtures must use
the current naming: every test in `proton_ge_test.rs`, `proton_ge_fetch_test.rs`
and `proton_ge_step_test.rs` used 11-3 naming, which matches the tag-derived
guess, so all of them stayed green while the real thing had been broken since
11-4 shipped.

Related: [[testing-a-tray-click-needs-the-cli-reinstalled]]
