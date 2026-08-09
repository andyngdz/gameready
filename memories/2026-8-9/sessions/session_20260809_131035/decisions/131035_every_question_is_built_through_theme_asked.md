# Every question is built through theme::Asked

Decision:
Decision: a CLI prompt is never constructed against `inquire` directly. `theme::Asked::new(question, detail, keys)` takes all three parts, and `one_of` / `any_of` return a `Select` / `MultiSelect` with the message, the render config, and the help line already set. `theme::Prompts` is private to `theme.rs`.

Reason: `inquire`'s `with_help_message` and `with_render_config` are optional builder calls, so a prompt with no keys line, or a one-of list drawn with the checkbox config, compiles and ships. Routing through `Asked` makes the compiler ask for all three and pairs each list shape with its own config.

`Asked` owns the rendered message as a `String` because `Select::new` borrows it for the prompt's lifetime, so a caller that builds its question dynamically binds the `Asked` to a local first:

```rust
let question = theme::Asked::new(&work.question(), &work.detail(), EITHER_WAY);
let answer = question.one_of(vec![...]).prompt_skippable()?;
```

Five callers: prompt.rs, overlay.rs, launch_choice.rs, install.rs, governor.rs.
