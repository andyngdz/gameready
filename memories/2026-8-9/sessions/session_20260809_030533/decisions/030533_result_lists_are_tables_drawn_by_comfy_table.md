# Result lists are tables, drawn by comfy-table

Decision:
Every screen that lists steps lays them out as a table with the name column pinned to `name_column(&short_names())`: the widest name in the catalog, not the widest in this run. Two runs of the same machine that settle different steps would otherwise line their evidence up at different columns.

`ui/layout/table.rs` wraps comfy-table (`ContentArrangement::Dynamic`, `NOTHING` style, `ColumnConstraint::Absolute`) for the screens rendered all at once: doctor, rollback, selftest, and the probe header. `Section::row` draws the same geometry one row at a time, for the live region, which prints a row as each step settles and cannot measure a table it has not finished, and for the summary, whose rows interleave with the blocks a failure gets. `table_test.rs` asserts the two land on the same column.

Two comfy-table facts that cost time:

- `Table::column_mut(n)` returns `None` until rows exist, so a constraint set on an empty table is dropped without a word. The table is therefore built at render time from stored rows, not filled as rows arrive.
- 8.0 renamed `load_preset` to `load_style`.

prettytable-rs was considered and rejected: last release 0.10.0 in 2022, an advisory-db entry requested, and it does not wrap cell content, which the scx_lavd skip reason (200+ characters) needs.
