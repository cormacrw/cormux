//! Engines with no process behind them, for tests: each thread's commands land in a receiver the
//! test holds, and the test can report as the engine.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::engines::{EngineCommand, EngineHandle, EngineOutput, Engines, LaunchConfig, Launching};
use crate::error::Error;

#[derive(Clone, Default)]
pub struct FakeEngines {
    state: Arc<Mutex<FakeState>>,
}

#[derive(Default)]
struct FakeState {
    commands: HashMap<String, Channel>,
    outputs: HashMap<String, mpsc::UnboundedSender<EngineOutput>>,
    /// Threads whose engines look alive but fail every send, the way one dying mid-prompt does.
    refused: HashSet<String>,
    launches: Vec<LaunchConfig>,
}

struct Channel {
    sender: mpsc::UnboundedSender<EngineCommand>,
    receiver: Option<mpsc::UnboundedReceiver<EngineCommand>>,
}

impl Channel {
    fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Some(receiver),
        }
    }
}

impl FakeEngines {
    /// What the thread's engines are sent, from the first launched for it on.
    pub fn engine(&self, thread_id: &str) -> mpsc::UnboundedReceiver<EngineCommand> {
        let mut state = self.state.lock().unwrap();
        let channel = state
            .commands
            .entry(thread_id.to_string())
            .or_insert_with(Channel::new);
        channel
            .receiver
            .take()
            .expect("one receiver per thread's engine")
    }

    pub fn refuse_sends(&self, thread_id: &str) {
        self.state.lock().unwrap().refused.insert(thread_id.into());
    }

    pub fn accept_sends(&self, thread_id: &str) {
        self.state.lock().unwrap().refused.remove(thread_id);
    }

    /// Reports as the thread's engine, once it has been launched.
    pub fn report(&self, thread_id: &str, output: EngineOutput) {
        let state = self.state.lock().unwrap();
        let sender = state.outputs.get(thread_id).expect("a launched engine");
        sender.send(output).expect("the runtime is reading");
    }

    /// The newest engine's way out, to report as it after it's been replaced.
    pub fn output(&self, thread_id: &str) -> mpsc::UnboundedSender<EngineOutput> {
        let state = self.state.lock().unwrap();
        state
            .outputs
            .get(thread_id)
            .expect("a launched engine")
            .clone()
    }

    /// Lets go of the newest engine's way out, so its output closes once the test's copies go.
    pub fn hang_up(&self, thread_id: &str) {
        self.state.lock().unwrap().outputs.remove(thread_id);
    }

    pub fn launches(&self) -> Vec<LaunchConfig> {
        self.state.lock().unwrap().launches.clone()
    }
}

impl Engines for FakeEngines {
    fn launch(
        &self,
        config: LaunchConfig,
        output: mpsc::UnboundedSender<EngineOutput>,
    ) -> Launching<'_> {
        Box::pin(async move {
            // Like a real engine, one can't start in a folder that isn't there.
            if !config.cwd.is_dir() {
                return Err(Error::Engine(format!(
                    "no folder at {}",
                    config.cwd.display()
                )));
            }
            let mut state = self.state.lock().unwrap();
            let thread_id = config.thread_id.clone();
            state.launches.push(config);
            state.outputs.insert(thread_id.clone(), output);
            let commands = if state.refused.contains(&thread_id) {
                mpsc::unbounded_channel().0
            } else {
                let channel = state.commands.entry(thread_id).or_insert_with(Channel::new);
                if channel.sender.is_closed() {
                    *channel = Channel::new();
                }
                channel.sender.clone()
            };
            Ok(EngineHandle {
                commands,
                pgid: None,
            })
        })
    }
}
