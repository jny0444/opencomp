use std::collections::VecDeque;

use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::{Model, Turn},
    observation::Observation,
};
use serde::Deserialize;

use crate::completer::Completer;
use crate::prompt;

const BURST_LIMIT: usize = 4;

pub struct SplitModel<P, G> {
    planner: P,
    grounder: G,
    frame_hash: fn(&[u8]) -> u64,
    subgoal: Option<String>,
    failures: u32,
    last_hash: Option<u64>,
    pending_ground: bool,
    burst: VecDeque<Action>,
}

impl<P, G> SplitModel<P, G> {
    pub fn new(planner: P, grounder: G, frame_hash: fn(&[u8]) -> u64) -> Self {
        Self {
            planner,
            grounder,
            frame_hash,
            subgoal: None,
            failures: 0,
            last_hash: None,
            pending_ground: false,
            burst: VecDeque::new(),
        }
    }
}

impl<P: Completer, G: Completer> Model for SplitModel<P, G> {
    async fn next_action(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        let hash = (self.frame_hash)(&observation.png);
        let unchanged =
            self.last_hash.is_some_and(|previous| previous == hash) && !history.is_empty();
        self.last_hash = Some(hash);

        if self.pending_ground {
            self.pending_ground = false;
            if unchanged {
                self.failures += 1;
                self.burst.clear();
            } else {
                self.subgoal = None;
                self.failures = 0;
            }
        } else if unchanged {
            self.failures += 1;
            self.burst.clear();
        }

        if !unchanged && let Some(action) = self.burst.pop_front() {
            tracing::info!(role = "burst", action = ?action);
            return Ok(turn(action, 0, None));
        }

        let missed = (unchanged || self.failures >= 2)
            .then(|| self.subgoal.clone())
            .flatten();
        let needs_plan =
            self.subgoal.is_none() || history.is_empty() || unchanged || self.failures >= 2;
        if needs_plan {
            if let Some(queued) = self
                .plan(task, observation, history, missed.as_deref())
                .await?
            {
                return Ok(queued);
            }
        }

        self.ground(task, observation, history).await
    }
}

impl<P: Completer, G: Completer> SplitModel<P, G> {
    async fn plan(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
        missed: Option<&str>,
    ) -> Result<Option<Turn>, OpenCompCoreError> {
        for attempt in 0..2 {
            let text = planner_prompt(
                task,
                observation.width,
                observation.height,
                history,
                missed,
                attempt == 1,
            )?;
            let reply = self.planner.complete(&text, Some(&observation.png)).await?;
            let mut plan: Plan = prompt::parse_json(&reply.text)?;
            if plan.subgoal.is_empty() && plan.actions.is_empty() {
                plan.actions = vec![prompt::parse_action(&reply.text)?];
            }
            plan.actions.truncate(BURST_LIMIT);
            let subgoal = plan.subgoal.clone();
            tracing::info!(
                role = "planner",
                subgoal = %subgoal,
                actions = plan.actions.len(),
                action = ?plan.actions.first()
            );
            let finishing = plan
                .actions
                .iter()
                .any(|action| matches!(action, Action::Done { .. }));
            let placeholder =
                matches!(plan.actions.as_slice(), [Action::Type(text)] if text == "text");
            let usable =
                !plan.actions.is_empty() && plan.actions.iter().all(is_text_action) && !placeholder;
            if subgoal.is_empty() && !finishing && !usable {
                if attempt == 0 {
                    continue;
                }
                self.failures = 0;
                self.subgoal = None;
                return Err(OpenCompCoreError::Model(
                    "planner returned no subgoal".to_owned(),
                ));
            }
            self.failures = 0;
            self.subgoal = if subgoal.is_empty() {
                None
            } else {
                Some(subgoal)
            };
            if plan.actions.is_empty() {
                return Ok(None);
            }
            if plan.actions.iter().all(is_text_action) {
                let mut actions = VecDeque::from(plan.actions);
                let action = actions.pop_front().expect("text burst has an action");
                self.burst = actions;
                return Ok(Some(turn(action, reply.output_bytes, reply.output_tokens)));
            }
            return Ok(None);
        }
        Err(OpenCompCoreError::Model(
            "planner returned no subgoal".to_owned(),
        ))
    }

    async fn ground(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        let mut replanned = false;
        loop {
            let subgoal = self
                .subgoal
                .clone()
                .filter(|subgoal| !subgoal.is_empty())
                .ok_or_else(|| {
                    OpenCompCoreError::Model("planner returned no subgoal".to_owned())
                })?;
            let text = grounder_prompt(&subgoal, observation.width, observation.height);
            let reply = self
                .grounder
                .complete(&text, Some(&observation.png))
                .await?;
            let ground = parse_ground(&reply.text)?;
            if ground.decline || ground.actions.is_empty() {
                tracing::info!(role = "grounder", subgoal = %subgoal, decline = true);
                self.failures += 1;
                if replanned {
                    return Err(OpenCompCoreError::Model(
                        "grounder found no target".to_owned(),
                    ));
                }
                replanned = true;
                if let Some(queued) = self
                    .plan(task, observation, history, Some(subgoal.as_str()))
                    .await?
                {
                    return Ok(queued);
                }
                continue;
            }
            let action = ground
                .actions
                .into_iter()
                .find(|action| !is_text_action(action))
                .ok_or_else(|| OpenCompCoreError::Model("grounder returned no point".to_owned()))?;
            tracing::info!(role = "grounder", subgoal = %subgoal, action = ?action);
            self.pending_ground = true;
            return Ok(turn(action, reply.output_bytes, reply.output_tokens));
        }
    }
}

fn turn(action: Action, output_bytes: usize, output_tokens: Option<u32>) -> Turn {
    turn_with(action, Vec::new(), output_bytes, output_tokens)
}

fn turn_with(
    action: Action,
    follow: Vec<Action>,
    output_bytes: usize,
    output_tokens: Option<u32>,
) -> Turn {
    Turn {
        action,
        follow,
        reasoning: None,
        output_bytes,
        output_tokens,
    }
}

fn is_text_action(action: &Action) -> bool {
    matches!(
        action,
        Action::Type(_)
            | Action::Key { .. }
            | Action::Scroll { .. }
            | Action::Wait { .. }
            | Action::Done { .. }
    )
}

fn planner_prompt(
    task: &str,
    width: u32,
    height: u32,
    history: &[Action],
    missed: Option<&str>,
    retry: bool,
) -> Result<String, OpenCompCoreError> {
    let mut text = format!(
        "Task: {task}\n\
         The screenshot is {width} by {height} pixels.\n\
         You are the planner. Reply with one JSON object and no markdown:\n\
         {{\"subgoal\":\"short phrase naming the next target\",\"actions\":[]}}\n\
         actions holds up to 4 steps that need no new screenshot: Type, Key, Scroll, Wait, or Done.\n\
         A Key action is {{\"Key\":{{\"keys\":[\"Enter\"]}}}}. Do not open Spotlight yourself.\n\
         Leave actions empty when the next step is a click, move, or drag. A grounder will pick the point.\n\
         Name a subgoal unless the actions finish with Done.\n\
         When the task is finished, actions is [{{\"Done\":{{\"result\":\"finished\"}}}}] and subgoal can be empty.\n"
    );
    if retry {
        text.push_str("The last subgoal was empty. Name the next target, or finish with Done.\n");
    }
    if let Some(missed) = missed {
        text.push_str(&format!(
            "The subgoal {missed:?} missed. Pick a different one.\n"
        ));
    }
    if !history.is_empty() {
        text.push_str("\nActions already performed:\n");
        for action in history {
            let line = serde_json::to_string(action)
                .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
            text.push_str(&line);
            text.push('\n');
        }
    }
    Ok(text)
}

fn grounder_prompt(subgoal: &str, width: u32, height: u32) -> String {
    format!(
        "Target: {subgoal}\n\
         The screenshot is {width} by {height} pixels.\n\
         You are the grounder. Reply with one JSON object and no markdown.\n\
         If the target is visible, return one point action in image pixels, for example\n\
         {{\"actions\":[{{\"Click\":{{\"point\":{{\"x\":10,\"y\":20}},\"button\":\"Left\"}}}}]}}\n\
         Move, DoubleClick, Drag, and Release use the same point shapes.\n\
         If the target is not visible, return {{\"decline\":true}}.\n\
         Do not type or press keys.\n"
    )
}

#[derive(Debug, Clone, Deserialize)]
struct Plan {
    #[serde(default)]
    subgoal: String,
    #[serde(default)]
    actions: Vec<Action>,
}

#[derive(Debug, Clone, Deserialize)]
struct Ground {
    #[serde(default)]
    decline: bool,
    #[serde(default)]
    actions: Vec<Action>,
}

fn parse_ground(text: &str) -> Result<Ground, OpenCompCoreError> {
    let parsed = prompt::parse_json::<Ground>(text);
    if let Ok(ground) = &parsed
        && (ground.decline || !ground.actions.is_empty())
    {
        return Ok(ground.clone());
    }
    if let Ok(action) = prompt::parse_action(text) {
        return Ok(Ground {
            decline: false,
            actions: vec![action],
        });
    }
    parsed
}
