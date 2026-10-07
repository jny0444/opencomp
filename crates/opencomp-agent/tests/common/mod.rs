use std::sync::{Arc, Mutex};

use opencomp_core::action::Action;
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_core::observation::Observation;

pub struct FakeComputer {
    seen: Arc<Mutex<Vec<Action>>>,
    width: u32,
    height: u32,
}

impl FakeComputer {
    pub fn new(width: u32, height: u32) -> (Self, Arc<Mutex<Vec<Action>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                seen: Arc::clone(&seen),
                width,
                height,
            },
            seen,
        )
    }
}

impl Computer for FakeComputer {
    async fn screenshot(&mut self) -> Result<Observation, OpenCompCoreError> {
        Ok(Observation {
            png: vec![0],
            width: self.width,
            height: self.height,
            screen_width: self.width,
            screen_height: self.height,
        })
    }

    async fn act(&mut self, action: &Action) -> Result<(), OpenCompCoreError> {
        self.seen.lock().unwrap().push(action.clone());
        Ok(())
    }
}
