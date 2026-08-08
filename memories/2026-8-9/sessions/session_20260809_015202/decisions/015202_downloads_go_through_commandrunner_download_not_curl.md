# Downloads go through CommandRunner::download, not curl

Decision:
`ProtonGe` fetches its tarball through `CommandRunner::download(url, dest, on_bytes)`, implemented with `ureq` in `infra/exec/download.rs`. It replaced `curl -sfLo` run as a command.

Reason: a spawned-and-waited process cannot say how far it got, and the progress bar needs the running total. ureq hands back a `Read`, so bytes are counted as they stream. The alternative that was tried first, a scoped thread running curl while the main thread polled the destination's size, needed a `file_size` method on the trait that had exactly one caller.

Why it is a trait method rather than a call to ureq inside the step: `FixtureRunner` has to be able to refuse it. A fixture stands in for a machine, and a fetch that reached the network anyway would make every screen taken against it a screen of something else. `MockRunner::serving(url, body)` seeds what a URL returns and reports it in two calls, so a test can tell reporting-as-it-goes from reporting-once-at-the-end.

`fetch_release` and the checksum fetch still go through `curl` as commands: they are small and their output is parsed, not streamed.
