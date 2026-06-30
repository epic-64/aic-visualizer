# token-visualizer

A terminal UI for simulating LLM conversations and visualizing their token
usage and cost. Describe each turn in plain language, pick a pricing model, and
watch the per-turn and cumulative cost build up as context accumulates.

## Requirements

- [Rust](https://rustup.rs/) (stable, edition 2024, Rust 1.85+)

## Running

```sh
cargo run
```

For an optimized build:

```sh
cargo run --release
```

Run the tests with:

```sh
cargo test
```

## Using the app

The app walks through a few screens:

1. **Model select**: pick a built-in model (Opus, Sonnet, Haiku, GPT-5, …) or
   press `c` to define a custom one with your own prices.
2. **Start**: begin a blank conversation, load a built-in example, or reopen a
   previously saved conversation.
3. **Chat**: type a description of each turn and press Enter to bill it.

### Describing a turn

Turns are free-form; the parser picks out the numbers and what they refer to.
Numbers may use `k`/`m` suffixes. Examples:

```
300 tokens prompt, 12000 tokens tool inputs, 4000 tokens response
300 prompt, 12k tool input, 4k response
input 1.5k, output 4k, cached 10k
500 reasoning, 1000 response
300 prompt, 5000 tools, 400 out, repeat 10
```

Keywords: `prompt`/`input`/`tools`/`instructions` → input, `out`/`response`/
`completion`/`answer` → output, `think`/`reason` → thinking (billed like output
but not carried into context), anything with `cach` → an explicit cached-token
override, and `repeat N` / `N times` to apply the same turn multiple times.

### Markers

Markers are positional annotations dropped between turns. Either send one on its
own line:

```
marker
marker: reviewed here
```

…or tuck one onto the end of a turn line; it is stripped from the stored turn
and a marker is placed right after it:

```
300 prompt, 400 out, marker: after review
```

### Other chat commands

- `clear`: wipe the current conversation.

## Keyboard shortcuts

**Global**

- `Ctrl-C`: quit

**Model select**

- `↑`/`↓` (or `k`/`j`): move · `Enter`: choose · `c`: custom model · `q`: quit

**Start picker**

- `↑`/`↓`: move · `Enter`: open · `d`/`Delete`: delete a saved conversation ·
  `Esc`: back

**Chat**

- `Enter`: submit turn · `↑`/`↓`: recall previous input lines · `Esc`: back to
  model select
- `Ctrl-S`: save conversation · `Ctrl-T`: new tab · `Ctrl-W`: close tab ·
  `Tab`/`Shift-Tab`: switch tabs
- Mouse wheel: scroll the turn history

## Saved conversations

Conversations are saved as plain-text files under
`~/.token-visualizer/conversations/`. Each file carries its model and prices in
`# key: value` headers followed by one turn (or marker) per line, so a saved
conversation restores itself completely on reload.
