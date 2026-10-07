# Building the computer-use agent

Follow the steps in order. Each step ends with a check that must pass before the next one starts.

The first finished version is a desktop agent: it screenshots the main display, asks a model for one action, performs that action, and repeats until the model says it is done or a step limit is hit. Browser control and OCR come after that loop works on a real screen.

`opencomp-core` is done. Start at Step 2.

## Crate map

| Crate | Kind | Depends on | Job |
|---|---|---|---|
| `opencomp-core` | library | nothing in this workspace | Actions, observations, errors, `Computer` and `Model` traits |
| `opencomp-computer` | library | `opencomp-core` | Screenshots (`xcap`) and mouse/keyboard (`enigo`) |
| `opencomp-llm` | library | `opencomp-core` | Turns a task and a screenshot into the next action |
| `opencomp-vision` | library | `opencomp-core` | Resize screenshots and map model coordinates back to screen pixels |
| `opencomp-agent` | library | `opencomp-core` | The observe → decide → act loop |
| `opencomp-cli` | binary | `opencomp-core`, `opencomp-agent`, `opencomp-computer`, `opencomp-llm` | Wires concrete drivers and runs a task |
| `opencomp-browser` | library | `opencomp-core` | Leave empty until the desktop loop works |

`opencomp-agent` depends only on `opencomp-core`. The CLI chooses the computer and the model. That keeps the loop testable without a display or an API key.

On macOS, grant the terminal Screen Recording and Accessibility in System Settings before the first real screenshot or click. `xcap` and `enigo` fail without those permissions.

Integration tests live in each crate's `tests/` directory. `cargo test -p <crate>` runs them. A dependency used from `tests/` must be listed under `[dev-dependencies]` as well as `[dependencies]`.

The error type in `opencomp-core` is `opencomp_core::error::OpenCompCoreError`. Use that name in every later crate.

## Step 1 — Shared types

Done. Leave these files alone unless a later step says to add one:

```text
crates/opencomp-core/src/lib.rs
crates/opencomp-core/src/action.rs
crates/opencomp-core/src/observation.rs
crates/opencomp-core/src/computer.rs
crates/opencomp-core/src/model.rs
crates/opencomp-core/src/error.rs
crates/opencomp-core/tests/action.rs
crates/opencomp-core/tests/error.rs
crates/opencomp-core/tests/traits.rs
```

`Action::Wait` stores `millis: u64`. `Done` is a signal to the agent. The computer driver must not perform it.

## Step 2 — Desktop driver

`crates/opencomp-computer/Cargo.toml` already depends on `opencomp-core`, `xcap`, `enigo`, and `image`. No new dependencies.

Create these files:

```text
crates/opencomp-computer/src/lib.rs
crates/opencomp-computer/src/desktop.rs
crates/opencomp-computer/src/input.rs
crates/opencomp-computer/src/keys.rs
crates/opencomp-computer/tests/keys.rs
crates/opencomp-computer/tests/desktop.rs
```

### `src/lib.rs`

Declare `desktop`, `input`, and `keys`. Re-export `Desktop`.

### `src/keys.rs`

One function:

```rust
pub fn to_enigo(key: opencomp_core::action::Key) -> enigo::Key
```

Map `Enter` to `enigo::Key::Return`, `Super` to `enigo::Key::Meta`, and `Char(c)` to `enigo::Key::Unicode(c)`. `Escape`, `Tab`, `Backspace`, `Delete`, `Alt`, `Control`, and `Shift` use the enigo variants with the same names.

### `src/input.rs`

Functions that take `&mut enigo::Enigo` and perform one action. `desktop.rs` calls them. Keep `xcap` out of this file.

Use `enigo::{Axis, Button, Coordinate, Direction, Keyboard, Mouse}`.

- `Click` and `DoubleClick`: `move_mouse(x, y, Coordinate::Abs)`, then `button(..., Direction::Click)`. Double-click calls `button` twice.
- `Move`: `move_mouse` only.
- `Drag`: move to `from`, `button(..., Direction::Press)`, move to `to`, `button(..., Direction::Release)`.
- `Release`: `move_mouse(x, y, Coordinate::Abs)`, then `button(..., Direction::Release)`.
- `Type`: `Keyboard::text`.
- `Key`: `key(..., Direction::Press)` for each key in order, then `key(..., Direction::Release)` in reverse order.
- `Scroll`: `scroll(dy, Axis::Vertical)` and `scroll(dx, Axis::Horizontal)`. Positive `dy` is down. Positive `dx` is right. Skip an axis whose delta is `0`.
- `Wait`: `std::thread::sleep(Duration::from_millis(millis))`.
- `Done` has no function here. `Desktop::act` returns `OpenCompCoreError::InvalidAction` and does not call enigo.

Map `opencomp_core::action::MouseButton` to `enigo::Button` here.

`enigo` on macOS posts Core Graphics points. `xcap` captures physical pixels. On a Retina display those differ by `Monitor::scale_factor()` (usually `2.0`). Divide `x` and `y` by that factor before `move_mouse`. Pass the factor into these functions from `Desktop`. When the factor is `1.0`, the coordinates pass through unchanged.

Turn enigo's `InputError` into `OpenCompCoreError::Computer` with `to_string()`.

### `src/desktop.rs`

```rust
pub struct Desktop {
    enigo: enigo::Enigo,
    scale_factor: f32,
}

impl Desktop {
    pub fn new() -> Result<Self, OpenCompCoreError>
}

impl Computer for Desktop {
    async fn screenshot(&mut self) -> Result<Observation, OpenCompCoreError>;
    async fn act(&mut self, action: &Action) -> Result<(), OpenCompCoreError>;
}
```

`new` builds `Enigo::new(&enigo::Settings::default())`.

`screenshot`:

1. `xcap::Monitor::all()` once, then reuse that primary monitor.
2. Keep the monitor whose `is_primary()` is true. If none is primary, use the first monitor.
3. `capture_image()`.
4. Sample the capture down so the longest side is at most 1280. Use nearest-neighbor. This is the image the model sees. Encode that with `PngEncoder::new_with_quality(..., CompressionType::Fast, FilterType::NoFilter)`.
5. Keep a 64×64 sample of the capture. When the next screenshot has the same sample and the same capture size, return the previous PNG instead of resampling and encoding.
6. Return `Observation { png, width, height, screen_width, screen_height }`. `width` and `height` are the PNG. `screen_width` and `screen_height` are the capture.
7. Store `monitor.scale_factor()` on `self`. `act` points are in the capture, and this factor converts those pixels to points.

Map `xcap::XCapError` into `OpenCompCoreError::Computer`.

`act` matches on `action` and calls `input`. Pass `self.scale_factor` into every call that moves the pointer. `Point` and `MouseButton` are `Clone`, so clone them when the helper takes them by value. `input::wait` returns `()`, so that arm is `Ok(())`.

```rust
match action {
    Action::Click { point, button } => {
        input::click(&mut self.enigo, point.clone(), button.clone(), self.scale_factor)
    }
    Action::DoubleClick(point) => {
        input::double_click(&mut self.enigo, point.clone(), self.scale_factor)
    }
    Action::Move(point) => input::move_to(&mut self.enigo, point.clone(), self.scale_factor),
    Action::Drag { from, to } => input::drag(
        &mut self.enigo,
        from.clone(),
        to.clone(),
        self.scale_factor,
    ),
    Action::Release { point, button } => input::release(
        &mut self.enigo,
        point.clone(),
        button.clone(),
        self.scale_factor,
    ),
    Action::Type(text) => input::type_text(&mut self.enigo, text),
    Action::Key { keys } => input::press_keys(&mut self.enigo, keys),
    Action::Scroll { dx, dy } => input::scroll(&mut self.enigo, *dx, *dy),
    Action::Wait { millis } => {
        input::wait(*millis);
        Ok(())
    }
    Action::Done { .. } => Err(OpenCompCoreError::InvalidAction(
        "done is not a computer action".to_owned(),
    )),
}
```

`Done` is the agent's stop signal. The same message is what `opencomp-core`'s trait test expects from a computer that is asked to perform it.

### Tests

`tests/keys.rs` is a normal unit test. Assert `Super` becomes `Meta`, `Enter` becomes `Return`, and `Char('a')` becomes `Unicode('a')`. This test does not move the mouse.

`tests/desktop.rs` is `#[ignore]`. It constructs `Desktop`, asserts the PNG is non-empty and `width` and `height` are non-zero, then `act`s a `Move` to a point near the origin. Run it with:

```text
cargo test -p opencomp-computer -- --ignored
```

Run that from a terminal that has Screen Recording and Accessibility.

**Done when:** `cargo test -p opencomp-computer` passes, and the ignored test captures a PNG and moves the mouse.

## Step 3 — Scripted model

No HTTP. Add no dependencies beyond `opencomp-core`.

Create these files:

```text
crates/opencomp-llm/src/lib.rs
crates/opencomp-llm/src/scripted.rs
crates/opencomp-llm/tests/scripted.rs
```

### `src/lib.rs`

Declare `scripted` and re-export `ScriptedModel`.

### `src/scripted.rs`

```rust
pub struct ScriptedModel {
    actions: Vec<Action>,
}

impl ScriptedModel {
    pub fn new(actions: Vec<Action>) -> Self
}

impl Model for ScriptedModel {
    async fn next_action(...) -> Result<Turn, OpenCompCoreError>;
}
```

Store the actions front-to-back. Each call removes the next one with `remove(0)` and returns it as `Turn { action, reasoning: None }`. Ignore `task`, `observation`, and `history`. When the vec is empty, return `OpenCompCoreError::Model`.

The last action in the vec the CLI passes should be `Done`. This type does not add `Done` itself.

### `tests/scripted.rs`

Build a model with `Move`, `Type`, and `Done`. Assert the third `next_action` is that `Done`. Assert the fourth call is `OpenCompCoreError::Model`. Use `tokio` as a dev-dependency and `#[tokio::test]`, the same way `opencomp-core` tests the traits.

**Done when:** `cargo test -p opencomp-llm` passes.

## Step 4 — Agent loop

Add workspace dep `tracing` to `crates/opencomp-agent/Cargo.toml`. The test crate also needs `tokio` under `[dev-dependencies]`.

Create these files:

```text
crates/opencomp-agent/src/lib.rs
crates/opencomp-agent/src/agent.rs
crates/opencomp-agent/tests/loop.rs
```

### `src/lib.rs`

Declare `agent` and re-export `Agent`.

### `src/agent.rs`

```rust
pub struct Agent<C, M> {
    computer: C,
    model: M,
    max_steps: usize,
}

impl<C, M> Agent<C, M> {
    pub fn new(computer: C, model: M, max_steps: usize) -> Self
}

impl<C: Computer, M: Model> Agent<C, M> {
    pub async fn run(&mut self, task: &str) -> Result<String, OpenCompCoreError>
}
```

`run` keeps a `Vec<Action>` named `history`. Each step:

1. `computer.screenshot()`.
2. `model.next_action(task, &observation, &history)`.
3. `tracing::info!` the step number (starting at 1) and the action.
4. On `Done { result }`, return `result`. Do not call `computer.act` and do not push `Done`.
5. Otherwise `computer.act(&action)`, push a clone of the action, and continue.
6. If `history.len()` reaches `max_steps` before `Done`, return `OpenCompCoreError::Model` with a message that the step limit was hit.

The CLI passes `25`. Tests pass a small limit.

### `tests/loop.rs`

Define a `FakeComputer` in this file. It returns a 1×1 PNG and pushes every `act` into a `Vec<Action>`. It must not special-case `Done`; the agent is what skips `Done`.

Use `ScriptedModel` from `opencomp-llm`. That means `opencomp-agent`'s dev-dependencies include `opencomp-llm` and `tokio`. The library dependency stays `opencomp-core` only.

Two tests:

- Script `Click` then `Done`. `run` returns the `Done` result. The fake computer recorded one click.
- Script a single `Click` and set `max_steps` to 1. `run` returns `OpenCompCoreError::Model`. The fake computer recorded one click.

**Done when:** `cargo test -p opencomp-agent` passes without a display or an API key.

## Step 5 — CLI

Add workspace deps `clap`, `tokio`, and `tracing-subscriber` to `crates/opencomp-cli/Cargo.toml`. `opencomp-core`, `opencomp-agent`, `opencomp-computer`, and `opencomp-llm` are already dependencies.

Create these files. `src/main.rs` already exists; replace it.

```text
crates/opencomp-cli/src/main.rs
crates/opencomp-cli/src/cli.rs
crates/opencomp-cli/src/demo.rs
```

### `src/cli.rs`

A clap parser:

```rust
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

pub enum Command {
    Run {
        #[arg(long)]
        task: String,
        #[arg(long, default_value = "scripted")]
        model: String,
        #[arg(long, default_value_t = 25)]
        max_steps: usize,
    },
}
```

Reject any `--model` value other than `scripted` with a clear error. The anthropic value arrives in Step 6.

### `src/demo.rs`

```rust
pub fn script() -> Vec<Action>
```

Return a short list you can edit while testing, ending in `Done { result }`. A move, a click, and `Done` is enough. This list is what `--model scripted` runs, so keep it harmless.

### `src/main.rs`

`#[tokio::main]`.

1. Parse `Cli`.
2. Install a `tracing_subscriber` fmt subscriber with `EnvFilter`. Default the filter to `info`.
3. Build `Desktop::new()`, `ScriptedModel::new(demo::script())`, and `Agent::new(..., max_steps)`.
4. `agent.run(&task).await`.
5. Print the result string to stdout. Print the error to stderr and exit with a non-zero status on failure.

**Done when:** `cargo run -p opencomp-cli -- run --task "demo" --model scripted` captures the screen, performs the demo actions, and prints the `Done` result.

## Step 6 — Real model and coordinate scaling

Do this only after Step 5 runs on your machine.

### Vision

Add workspace deps `image` and `base64` to `crates/opencomp-vision/Cargo.toml`. Tests need `image` as a dev-dependency too.

Create these files:

```text
crates/opencomp-vision/src/lib.rs
crates/opencomp-vision/src/scale.rs
crates/opencomp-vision/tests/scale.rs
```

### `src/lib.rs`

Declare `scale` and re-export `scale_screenshot`, `to_screen`, and `scale_action`.

### `src/scale.rs`

```rust
pub struct ScaledImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn scale_screenshot(png: &[u8], max_width: u32) -> Result<ScaledImage, OpenCompCoreError>

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

`scale_screenshot` decodes the PNG, shrinks it so the longest side is at most `max_width`, and keeps the aspect ratio. If the image is already smaller, return it unchanged. Re-encode as PNG. `width` and `height` are the size the model will see.

`to_screen` maps a point in that resized image onto the screenshot's pixel size:

```text
screen_x = point.x * screen_width / model_width
screen_y = point.y * screen_height / model_height
```

Use `u64` intermediates so the multiply does not overflow. Clamp to `screen_width - 1` and `screen_height - 1` when those dimensions are non-zero.

`scale_action` clones the action and runs `to_screen` on every point in `Click`, `DoubleClick`, `Move`, `Drag`, and `Release`. Leave `Type`, `Key`, `Scroll`, `Wait`, and `Done` unchanged.

### `tests/scale.rs`

Build a solid image with the `image` crate, encode it to PNG, and scale it to half the width. Assert the returned size. A point at the bottom-right of that half-size image must land on the bottom-right pixel of the original size. A `Type` action must come back unchanged from `scale_action`.

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
- Coordinates in the action are in that image, not the raw screen.
- The reply is one JSON value matching `Action`, with no markdown fence.

Include one JSON example taken from `crates/opencomp-core/tests/action.rs`, such as the left-click object.

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

The screenshot passed into `next_action` is already the scaled image. This crate does not call `opencomp-vision`. Scaling stays in the CLI so `opencomp-llm` does not depend on `opencomp-vision`.

### `tests/anthropic.rs`

Use `wiremock` to stand up a fake Anthropic response whose text is `{"Done":{"result":"ok"}}`. Point the client at that server. Assert `next_action` returns that `Done`. A second test returns a body that is not JSON and asserts `OpenCompCoreError::Model`.

### CLI changes

Add `dotenvy` and `opencomp-vision` to `opencomp-cli`.

In `src/cli.rs`, accept `--model anthropic` as well as `scripted`.

In `src/main.rs`, for `anthropic`:

1. `dotenvy::dotenv().ok()`.
2. Read `ANTHROPIC_API_KEY`. Missing key is a startup error.
3. Pass the screenshot straight to `next_action`. `Desktop` already keeps its PNG at most 1280 on the long side.
4. Run `scale_action` on the returned action before `computer.act`. The model size is `observation.width` and `observation.height`. The screen size is `observation.screen_width` and `observation.screen_height`.

`Desktop::act` still receives capture pixels and divides by `scale_factor` itself. The scripted path skips `scale_action`. Its actions are already capture pixels.

**Done when:** `cargo test -p opencomp-vision` and `cargo test -p opencomp-llm` pass, and a short real task finishes with `Done` with the typed text visible on screen.

## Step 7 — Safety limits

Add this in `opencomp-agent` before longer tasks. No new dependencies.

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

- For every point in `Click`, `DoubleClick`, `Move`, `Drag`, and `Release`, return `InvalidAction` when `x >= width` or `y >= height`.
- For `Key`, return `InvalidAction` when the chord contains `Super` and `Char('q')` or `Char('Q')`.
- Allow `Type`, `Scroll`, `Wait`, and `Done`.
- Allow a `Key` chord that is a single key, or `Super` combined with one of `Char('c')`, `Char('v')`, `Char('a')`, `Char('t')`, `Char('w')`, `Tab`.
- Any other chord is `InvalidAction`.

Call `policy::check` in `Agent::run` after the model returns and before `computer.act`. On `Done`, return the result before the policy check. Log every refusal with `tracing::warn!` and return the error. Do not continue the loop after a refusal.

### `tests/policy.rs`

A point with `x == width` is refused, and the fake computer from `tests/loop.rs` records nothing. Move the fake into `tests/common/mod.rs` if both test crates need it. Cargo integration tests do not share modules across files, so put the fake in `tests/common/mod.rs` and add `mod common;` at the top of `loop.rs` and `policy.rs`.

A `Super` + `Char('q')` chord is refused. A left click inside the screenshot is allowed.

**Done when:** `cargo test -p opencomp-agent` passes, including the out-of-bounds point test.

## Step 8 — Browser

Start `opencomp-browser` only after a desktop task succeeds with a real model.

Add workspace dep `playwright-rs` to `crates/opencomp-browser/Cargo.toml`.

Create these files:

```text
crates/opencomp-browser/src/lib.rs
crates/opencomp-browser/src/session.rs
crates/opencomp-browser/tests/session.rs
```

### `src/session.rs`

Implement `Computer` for a `Session` that owns a Playwright page.

- `screenshot` uses the page screenshot, and sets `width` and `height` from the viewport.
- `act` uses the page mouse and keyboard for the same `Action` variants as `Desktop`.
- `Done` returns `InvalidAction`.

Pixel actions stay on the `Computer` trait, so `Agent` does not change. Add `crates/opencomp-core/src/browser.rs` and a `Browser` trait only if you replace coordinates with locators. Export it from `lib.rs` if you add it.

### `tests/session.rs`

`#[ignore]`. Launch a page, screenshot it, and assert a non-empty PNG. Run it only when Playwright's browsers are installed.

### CLI

Add `opencomp-browser` to `opencomp-cli` when this step starts.

Add `--computer` to the `Run` command in `src/cli.rs`, with values `desktop` (default) and `browser`. `main.rs` builds either `Desktop` or `Session` and passes it to `Agent`. Leave `--computer desktop` working.

**Done when:** `cargo run -p opencomp-cli -- run --task "demo" --model scripted --computer browser` screenshots a page and runs the demo script, and `--computer desktop` still works.

## Step 9 — OCR

Add this only when a model cannot read the screenshot. No action types change.

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

Re-export `read` and `TextBlock` from `src/lib.rs`. Write the PNG to a tempfile, run Tesseract, and return one block per recognized word or line. Coordinates are in the image that was passed in. The caller scales them with `to_screen` if that image is the resized screenshot.

`tests/ocr.rs` can be `#[ignore]` unless Tesseract is installed. Feed it a PNG with a known word and assert that word appears in some block.

The agent does not call `read` until you decide the model needs the text. When you do, append the blocks to the prompt built in `opencomp-llm/src/prompt.rs`.

## v1 is done when

- `cargo test --workspace` passes.
- `opencomp run --model scripted` finishes on a machine with no API key.
- `opencomp run --model anthropic` completes one desktop task you can see.
- A point outside the screenshot is refused.
- `opencomp-browser` is still unused by the CLI until Step 8.
