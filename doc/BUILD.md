# Building the computer-use agent

Follow the steps in order. Each step ends with a check that must pass before the next one starts.

The first finished version is a desktop agent: it screenshots the main display, asks a model for one action, performs that action, and repeats until the model says it is done or a step limit is hit. Browser control and OCR come after that loop works on a real screen.

Steps 1–6 are done. Start at Step 7.

## Crate map

| Crate | Kind | Depends on | Job |
|---|---|---|---|
| `opencomp-core` | library | nothing in this workspace | Actions, observations, errors, `Computer` and `Model` traits |
| `opencomp-computer` | library | `opencomp-core` | Screenshots (`xcap`) and mouse/keyboard (`enigo`) |
| `opencomp-llm` | library | `opencomp-core` | Turns a task and a screenshot into the next action |
| `opencomp-vision` | library | `opencomp-core` | Empty until Step 9. Desktop screenshots are already resized |
| `opencomp-agent` | library | `opencomp-core` | The observe → decide → act loop |
| `opencomp-cli` | binary | `opencomp-core`, `opencomp-agent`, `opencomp-computer`, `opencomp-llm` | Wires concrete drivers and runs a task |
| `opencomp-browser` | library | `opencomp-core` | Leave empty until the desktop loop works with a real model |

`opencomp-agent` depends only on `opencomp-core`. The CLI chooses the computer and the model. That keeps the loop testable without a display or an API key.

On macOS, grant the terminal Screen Recording and Accessibility in System Settings before the first real screenshot or click. `xcap` and `enigo` fail without those permissions.

Integration tests live in each crate's `tests/` directory. `cargo test -p <crate>` runs them. A dependency used from `tests/` must be listed under `[dev-dependencies]` as well as `[dependencies]`.

The error type in `opencomp-core` is `opencomp_core::error::OpenCompCoreError`. Use that name in every later crate.

## Step 1 — Shared types

Done.

`Action` includes `Release { point, button }` along with click, double-click, move, drag, type, key, scroll, wait, and done. `Action::Wait` stores `millis: u64`. `Done` is a signal to the agent. The computer driver must not perform it.

`Observation` is the model image plus the capture size:

```rust
pub struct Observation {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub screen_width: u32,
    pub screen_height: u32,
}
```

`width` and `height` are the PNG. `screen_width` and `screen_height` are the captured screen. `Desktop::act` points are in that capture.

## Step 2 — Desktop driver

Done. `cargo test -p opencomp-computer` passes. `tests/desktop.rs` is `#[ignore]` and is the manual check that a real screenshot is a PNG whose long side is at most 1280 and that a `Move` near the origin runs. Run it from a terminal that has Screen Recording and Accessibility:

```text
cargo test -p opencomp-computer -- --ignored
```

What landed:

- `keys::to_enigo` maps `Enter` to `Return`, `Super` to `Meta`, and `Char(c)` to `Unicode(c)`.
- `input` performs every action except `Done`. Pointer actions divide by `scale_factor` before `move_mouse`. `Release` moves, then sends `Direction::Release`.
- `Desktop::act` returns `OpenCompCoreError::InvalidAction("done is not a computer action")` for `Done`.
- `screenshot` keeps the primary monitor. If none is primary, it returns `OpenCompCoreError::Computer`. It does not fall back to the first monitor.
- The PNG is a nearest-neighbor sample of the capture, longest side at most 1280, stored as opaque RGB with fast compression and a Sub filter. A 64×64 sample of the capture is cached; an unchanged screen returns the previous PNG.
- `scale_factor` stays the monitor scale. `act` still receives capture pixels, not PNG pixels.

## Step 3 — Scripted model

Done. `cargo test -p opencomp-llm` passes.

`ScriptedModel::new` stores actions front-to-back. Each `next_action` returns `remove(0)` as `Turn { action, reasoning: None }`. An empty vec is `OpenCompCoreError::Model("no scripted action left")`. It does not append `Done`.

## Step 4 — Agent loop

Done. `cargo test -p opencomp-agent` passes without a display or an API key.

`Agent::run` screenshots, asks the model, logs the step starting at 1, and either returns `Done`'s result or calls `act` and pushes the action. `history.len() == max_steps` before `Done` returns `OpenCompCoreError::Model` with `step limit of {n} was hit`. The fake computer records every `act` and does not special-case `Done`.

`run` does not scale coordinates yet. Scripted points are capture pixels. Step 6 changes that.

## Step 5 — CLI

Done. The code matches this step.

`opencomp-cli run --task ... --model scripted` builds `Desktop`, `ScriptedModel::new(demo::script())`, and `Agent` with `--max-steps` defaulting to 25. Any other `--model` exits with an error. The result is printed to stdout. Errors go to stderr with a non-zero status.

`demo::script` is a move, a click, a release, another move, and `Done`. Those points are capture pixels until Step 6.

Confirm on a machine with Screen Recording and Accessibility:

```text
cargo run -p opencomp-cli -- run --task "demo" --model scripted
```

## Step 6 — Real model and coordinate mapping

Done. `cargo test -p opencomp-core`, `cargo test -p opencomp-llm`, and `cargo test -p opencomp-agent` pass. A real Anthropic task still has to be run on your machine.

Do not add a second resize. `Desktop::screenshot` already returns a PNG whose longest side is at most 1280. `scale_screenshot` would shrink that image again and send the model the wrong coordinate space. Leave `opencomp-vision` empty until Step 9.

Do not scale in `main` around `agent.run`. `Agent::run` calls `computer.act` itself, so `main` never sees the action. Map coordinates inside `run`.

### Mapping

Add these to `opencomp-core`. They are integer arithmetic. They do not need the `image` crate, so `opencomp-agent` can call them and still depend only on `opencomp-core`.

```rust
pub fn to_screen(
    point: Point,
    model_width: u32,
    model_height: u32,
    screen_width: u32,
    screen_height: u32,
) -> Point

pub fn scale_action(
    action: &Action,
    model_width: u32,
    model_height: u32,
    screen_width: u32,
    screen_height: u32,
) -> Action
```

```text
screen_x = point.x * screen_width / model_width
screen_y = point.y * screen_height / model_height
```

Use `u64` intermediates so the multiply does not overflow. Clamp to `screen_width - 1` and `screen_height - 1` when those dimensions are non-zero. When `model_width` or `model_height` is `0`, return the point unchanged.

`scale_action` clones the action and runs `to_screen` on every point in `Click`, `DoubleClick`, `Move`, `Drag`, and `Release`. Leave `Type`, `Key`, `Scroll`, `Wait`, and `Done` unchanged.

In `Agent::run`, after `next_action` and after the `Done` check:

1. Push a clone of the model action onto `history`. That clone stays in PNG pixels, which is the space the next prompt must describe.
2. `scale_action` from `observation.width` / `observation.height` to `observation.screen_width` / `observation.screen_height`.
3. `computer.act` the scaled action.

The fake computer in `tests/loop.rs` uses a 1×1 image whose screen size equals the PNG size, so the map is a no-op and the existing tests stay valid.

`demo::script` must use PNG pixels too, the same space Anthropic will use. Points near the origin can stay near the origin. They are no longer raw capture pixels.

### Tests

In `opencomp-core`, a point at the bottom-right of a half-size image lands on the bottom-right pixel of the full size. A `Type` action comes back unchanged from `scale_action`.

### Model client

Add workspace deps `reqwest`, `serde_json`, and `base64` to `crates/opencomp-llm`. Add `wiremock` as a dev-dependency.

Create these files. Leave `scripted.rs` in place.

```text
crates/opencomp-llm/src/anthropic.rs
crates/opencomp-llm/src/prompt.rs
crates/opencomp-llm/tests/anthropic.rs
```

Re-export `AnthropicModel` from `src/lib.rs`.

### `src/prompt.rs`

```rust
pub fn instruction(task: &str, image_width: u32, image_height: u32) -> String
```

The string tells the model:

- The task text.
- The image is `image_width` by `image_height`.
- Coordinates in the action are in that image, not `screen_width` by `screen_height`.
- The reply is one JSON value matching `Action`, with no markdown fence. List every `Action` shape. A task that moves and then stops is `Move` on this reply and `Done` on a later reply. Do not invent names.

### `src/anthropic.rs`

```rust
pub struct AnthropicModel {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicModel {
    pub fn new(api_key: String, model: String) -> Self
}

impl Model for AnthropicModel {
    async fn next_action(...) -> Result<Turn, OpenCompCoreError>
}
```

`next_action` sends the instruction, the PNG as a base64 image, and the action history. Parse the response text with `serde_json::from_str::<Action>`. On success return `Turn { action, reasoning: None }`. On HTTP failure or JSON failure return `OpenCompCoreError::Model`.

Pass `observation.png` through unchanged. This crate does not resize it.

### `tests/anthropic.rs`

Use `wiremock` to stand up a fake Anthropic response whose text is `{"Done":{"result":"ok"}}`. Point the client at that server. Assert `next_action` returns that `Done`. A second test returns a body that is not JSON and asserts `OpenCompCoreError::Model`.

### CLI changes

Add `dotenvy` to `opencomp-cli`. Do not add `opencomp-vision` for this step.

In `src/cli.rs`, accept `--model anthropic`, `groq`, and `openrouter` as well as `scripted`.

In `src/main.rs`:

1. `dotenvy::dotenv().ok()` for every provider.
2. `anthropic` reads `ANTHROPIC_API_KEY` and optional `ANTHROPIC_MODEL` (default `claude-sonnet-4-5`).
3. `groq` reads `GROQ_API_KEY` and optional `GROQ_MODEL` (default `qwen/qwen3.8-27b`, which can see images) and calls the Groq chat completions API.
4. `openrouter` reads `OPENROUTER_API_KEY` and optional `OPENROUTER_MODEL` (default `qwen/qwen3.8-27b`).
5. A missing key is a startup error. `Agent::run` scales the action before `act`. Anthropic receives `observation.png`. Groq and OpenRouter re-encode that PNG as JPEG, starting at quality 75 and stepping down to 60 and 45 while the file is over 700KB, because OpenRouter rejects the uncompressed PNG with HTTP 413. The JPEG keeps the PNG's width and height.

The scripted path uses the same `run`. Its demo points are PNG pixels, and `run` scales those too.

**Done when:** `cargo test -p opencomp-core` and `cargo test -p opencomp-llm` pass, and a short real task finishes with `Done` with the typed text visible on screen.

## Step 7 — Safety limits

Follow this before longer tasks. No new dependencies.

Create these files:

```text
crates/opencomp-agent/src/policy.rs
crates/opencomp-agent/tests/policy.rs
```

Declare `policy` from `src/lib.rs`.

### `src/policy.rs`

```rust
pub fn check(action: &Action, width: u32, height: u32) -> Result<(), OpenCompCoreError>
```

Check the model action, before `scale_action`. `width` and `height` are `observation.width` and `observation.height`, the image the model saw.

- For every point in `Click`, `DoubleClick`, `Move`, `Drag`, and `Release`, return `InvalidAction` when `x >= width` or `y >= height`.
- For `Key`, return `InvalidAction` when the chord contains `Super` and `Char('q')` or `Char('Q')`.
- Allow `Type`, `Scroll`, `Wait`, and `Done`.
- Allow a `Key` chord that is a single key, or `Super` combined with one of `Char('c')`, `Char('v')`, `Char('a')`, `Char('t')`, `Char('w')`, `Tab`.
- Any other chord is `InvalidAction`.

Call `policy::check` in `Agent::run` after the model returns and before `scale_action`. On `Done`, return the result before the policy check. Log every refusal with `tracing::warn!` and return the error. Do not continue the loop after a refusal. Do not push a refused action onto `history`.

### `tests/policy.rs`

A point with `x == width` is refused, and the fake computer records nothing. Move the fake into `tests/common/mod.rs` if both test crates need it. Cargo integration tests do not share modules across files, so put the fake in `tests/common/mod.rs` and add `mod common;` at the top of `loop.rs` and `policy.rs`.

A `Super` + `Char('q')` chord is refused. A left click inside the screenshot is allowed.

**Done when:** `cargo test -p opencomp-agent` passes, including the out-of-bounds point test.

## Step 8 — Browser

Follow this only after a desktop task succeeds with a real model.

Add workspace dep `playwright-rs` to `crates/opencomp-browser/Cargo.toml`.

Create these files:

```text
crates/opencomp-browser/src/lib.rs
crates/opencomp-browser/src/session.rs
crates/opencomp-browser/tests/session.rs
```

### `src/session.rs`

Implement `Computer` for a `Session` that owns a Playwright page.

- `screenshot` uses the page screenshot. Set `width` and `height` from the PNG the model will see, and `screen_width` and `screen_height` from the viewport. If you sample it down the way `Desktop` does, the PNG stays at most 1280 on the long side. If the PNG is the viewport, the two sizes are equal.
- `act` uses the page mouse and keyboard for the same `Action` variants as `Desktop`, including `Release`. Points passed to `act` are viewport pixels, because `Agent::run` has already scaled them.
- `Done` returns `InvalidAction`.

Pixel actions stay on the `Computer` trait, so `Agent` does not change. Add `crates/opencomp-core/src/browser.rs` and a `Browser` trait only if you replace coordinates with locators. Export it from `lib.rs` if you add it.

### `tests/session.rs`

`#[ignore]`. Launch a page, screenshot it, and assert a non-empty PNG. Run it only when Playwright's browsers are installed.

### CLI

Add `opencomp-browser` to `opencomp-cli` when this step starts.

Add `--computer` to the `Run` command in `src/cli.rs`, with values `desktop` (default) and `browser`. `main.rs` builds either `Desktop` or `Session` and passes it to `Agent`. Leave `--computer desktop` working.

**Done when:** `cargo run -p opencomp-cli -- run --task "demo" --model scripted --computer browser` screenshots a page and runs the demo script, and `--computer desktop` still works.

## Step 9 — OCR

Do not start this with Step 6. Add it only when a model cannot read the screenshot. No action types change.

Add workspace dep `leptess` to `crates/opencomp-vision`. `leptess` needs Tesseract installed on the machine.

Create:

```text
crates/opencomp-vision/src/ocr.rs
crates/opencomp-vision/tests/ocr.rs
```

### `src/ocr.rs`

```rust
pub struct TextBlock {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn read(png: &[u8]) -> Result<Vec<TextBlock>, OpenCompCoreError>
```

Re-export `read` and `TextBlock` from `src/lib.rs`. Write the PNG to a tempfile, run Tesseract, and return one block per recognized word or line. Coordinates are in the image that was passed in. That image is the model PNG, so scale the blocks with `to_screen` before using them as capture pixels.

`tests/ocr.rs` can be `#[ignore]` unless Tesseract is installed. Feed it a PNG with a known word and assert that word appears in some block.

The agent does not call `read` until you decide the model needs the text. When you do, append the blocks to the prompt built in `opencomp-llm/src/prompt.rs`.

## v1 is done when

- `cargo test --workspace` passes.
- `opencomp run --model scripted` finishes on a machine with no API key.
- `opencomp run --model anthropic` completes one desktop task you can see.
- A point outside the model image is refused.
- `opencomp-browser` is still unused by the CLI until Step 8.
