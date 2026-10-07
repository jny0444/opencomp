# Making opencomp fast

Read this after `BUILD.md`. v1 is the right loop: screenshot, one model call, one action, repeat. This guide changes how often that loop calls a model, and how expensive each call is. Leave the loop's shape alone until the measurements below say otherwise.

Speed here is wall time for a whole task. A frontier vision call is typically 1–4 seconds. Fifteen of those is a minute, and one wrong click adds another full round trip plus recovery. A fast clone keeps routine steps under a few hundred milliseconds and calls the big model only when the plan changes.

## Where the time goes

On a Retina display the unscaled capture is often about 3000×2000. Sent as PNG, that image dominates prefill. Output length dominates decode. A short chain of thought can cost more than the click.

| Stage | v1 | Target |
|---|---|---|
| Capture and encode | 50–200 ms | 10–30 ms |
| Frontier planner | 1–4 s, every step | 0.8–2 s, a few times per task |
| Local grounder | none | 80–200 ms per click |
| Act and UI settle | 50–400 ms, often a blind `Wait` | 50–200 ms, stop when pixels change |

`Agent::run` in `opencomp-agent` is already the right skeleton. Enigo, the Rust loop, and PNG micro-optimizations are small next to one model call. Spend the effort on the model row.

## Order of work

Do these in order. Each step has a check. Skip ahead and you will tune the wrong stage.

1. Measure a real task.
2. Shrink what the model sees.
3. Execute a burst of actions per call.
4. Use a structure (accessibility tree or browser DOM) when the target has one.
5. Split planning from grounding.
6. Speculate the next action only after the success rate is already high.

## Step 1 — Measure

No new behavior. Add timing so a real run shows, per step:

- capture milliseconds
- encoded image bytes and pixel size
- model milliseconds
- output size (response bytes, or token counts if the API returns them)
- whether the frame hash changed after `act`

Put the logs in `Agent::run` with `tracing`, around `screenshot`, `next_action`, and `act`. Hash a downscaled gray buffer of the PNG in `opencomp-vision` so the agent stays free of image code. The CLI already installs a subscriber.

**Done when:** one `anthropic` desktop task prints a line per step, and the model column is almost the whole sum. If it is not, fix that stage first and come back.

## Step 2 — Starve the image

The screenshot passed into `next_action` is the vision prefill. Make it small and send it once.

In `opencomp-computer`:

- `Desktop::screenshot` already samples the capture down to a longest side of 1280 and reuses that PNG while a 64×64 sample of the screen is unchanged. `screen_width` and `screen_height` stay the capture size. `act` still converts capture pixels to points with `scale_factor`.
- Capture the frontmost window when it is available. Fall back to the primary monitor.

In the CLI, `observation.png` is already the model image:

- Drop the longest side to 1024 only if Step 1 shows prefill still dominating and clicks stay accurate.
- Encode JPEG or WebP around quality 75 for the model payload. Keep a PNG only where a crate still requires one.
- Send the latest frame only. `Model::next_action` already takes one `Observation` plus `&[Action]`. Keep history as text.

In `opencomp-llm`:

- Keep the reply to one JSON `Action` (later, a short list). Ask for no reasoning trace. `Turn.reasoning` can stay empty.
- Cache the stable instruction on the provider. The task and the image change; the rules do not.

`opencomp-vision` still maps model coordinates back to screen pixels with `scale_action` before `computer.act`.

**Done when:** a repeated click task shows a clear drop in model time and image bytes against the Step 1 log, and the click still lands.

## Step 3 — Bursts

One call should return every action that does not need a new screenshot.

A search is three local actions and one capture: click the field, type the query, press Enter. Re-querying between those is the usual reason a demo feels slow.

Extend the model reply from one `Action` to a short `Vec<Action>`, capped (start at 4). `Done` ends the burst and the task. `Wait` is a signal to stop and look, and should be rare.

In `Agent::run`, for each action in the burst:

1. `policy::check` against the latest screenshot size.
2. `computer.act`.
3. Push it onto `history`.
4. After the burst, screenshot again.

Settle on a frame hash from Step 1. After an action that should change the UI, poll until the hash changes or a short cap (about 300 ms) elapses. An unchanged frame means the action missed. Log that and go back to the model with the same screen. A model-chosen `Wait { millis }` sleeps in the dark.

The scripted model returns its script as bursts too, so `cargo test -p opencomp-agent` still needs no display and no API key.

**Done when:** a task that types into a focused field uses one model call for click, type, and key, and the trace shows a single model time for that sequence.

## Step 4 — Structure before pixels

Pixels are the fallback for targets that expose no tree. A box from the OS or the DOM is a better click than a coordinate, and a miss costs a whole extra model call.

**Browser.** `BUILD.md` Step 8 keeps pixel actions on `Computer`, which is the right v1. After that works, add a locator path for pages that have a DOM: fill, click, and press through Playwright, and screenshot only for canvas-like pages. Route `--computer browser` at web tasks. A DOM action is milliseconds and does not drift with DPI.

**macOS.** Read the accessibility tree of the frontmost app inside `opencomp-computer` (or a sibling module the CLI selects). When a control has a frame, click the frame. When it does not, fall back to the screenshot path.

**Set-of-marks.** When you do send pixels, draw numbered boxes from the tree (or from OCR blocks in `opencomp-vision`) and have the model return an id. Map the id to a `Point` before `act`. The agent still sees an `Action`.

`policy::check` still runs on the point you are about to send.

**Done when:** one browser task and one native task complete with the trace showing tree or DOM hits, and a no-tree target still completes through the screenshot path.

## Step 5 — Planner and grounder

Put two models behind `Model`, so `Agent` stays observe → decide → act.

- **Planner.** The frontier model. Call it at the start, and again when a check fails or the grounder reports a miss. Input is the task, the text history, and at most one screenshot. Output is the next subgoal as a short phrase ("the Spotlight text field"), plus a burst when the actions are obvious from text alone.
- **Grounder.** A small GUI model, on-device if you can (a quantized 2B–7B UI model). Input is the latest frame and one phrase. Output is a point or a burst of points. Budget 80–200 ms.

The CLI owns the split, the same way it owns `scale_screenshot`. `opencomp-llm` can hold both clients. `opencomp-agent` keeps depending only on `opencomp-core`.

Call the planner when:

- the task has just started
- the frame hash did not change after an action
- the same subgoal fails twice
- the grounder declines (no target in the crop)

Otherwise call the grounder.

**Done when:** a multi-step desktop task shows a handful of planner calls and a grounder call per click, and the task still ends in `Done`.

## Step 6 — Speculate

Do this last. After each executed action, a small model guesses the next action from the previous frame plus the action just taken. Compare that guess to a real decision on the new screenshot.

- Match: execute it and skip a round trip.
- Miss: discard it and use the grounder result.

Keep the check. A wrong speculative click is slower than the round trip you saved.

**Done when:** the trace marks speculative hits and misses, the hit path shortens the task, and the miss path still matches the Step 5 behavior.

## What to leave in place

- `Agent` as a single loop over screenshot → model → policy → act. Concurrency inside that loop waits on the model either way.
- One image in the request. Earlier screenshots belong in the log, not the prompt.
- Text-only action history.
- Coordinate scaling in the CLI, and point-to-pixel conversion in `Desktop`.
- `Done` handled by the agent, refused by every `Computer`.
- The safety checks in `policy.rs`, applied to every action before `act`.

## Targets

After Steps 1–5, a normal click lands around 200–400 ms, and a frontier call happens a few times per task. Step 6 trims the grounder calls that are predictable from the previous action.

If a task is still slow, the log from Step 1 names the row. Another capture format or a faster mouse driver will not move that number while the model column is the wide one.
