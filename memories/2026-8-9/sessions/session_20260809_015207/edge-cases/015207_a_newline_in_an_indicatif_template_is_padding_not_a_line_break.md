# A newline in an indicatif template is padding, not a line break

Case:
`ProgressStyle::with_template("...{msg}\n    [{bar}]...")` does not render as two lines in indicatif 0.18.6. The bar is padded out to the terminal width instead, which puts it hard against the right margin with a field of blank between it and the message.

Verified by rendering it: `crates/gameready-cli/src/cli/ui/region.rs` now uses a single-line template.

Two more things that only show up when you look at it:

- `{bytes}` is binary (`178.05 MiB`). Every other gameready screen sizes a download in MB, so the template uses `{decimal_bytes}`/`{decimal_total_bytes}`.
- `finish_with_message` on a bar whose style still contains `{bar}` leaves the full bar trailing the finished row. `LiveRegion::settle` clears a counting bar and returns false so the caller prints the row itself.
