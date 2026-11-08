use std::time::Duration;

use tokio::time::Instant;

use super::protocol::Event;
use crate::engines::events::AgentEvent;
use crate::model::StopReason;

/// A finished agent starts its follow-up turn at once, but a stopped one starts none, so its
/// thread can't wait for another `result`.
const FOLLOW_UP_GRACE: Duration = Duration::from_secs(5);

/// Claude reports a turn's `result` while background subagents are still running, and each one
/// that finishes starts a turn of its own. This keeps a turn's end back until they are done, so
/// the thread doesn't read as idle while it is still working.
///
/// It also counts the prompts the CLI hasn't taken. One sent mid-turn is taken into that turn
/// at its next tool call, or else starts a turn of its own after this one's `result`; the CLI
/// echoes each as it takes it, so a turn that ends with one untaken has another coming.
#[derive(Default)]
pub struct TurnGate {
    agents: Vec<String>,
    held: Option<AgentEvent>,
    release_at: Option<Instant>,
    untaken: usize,
}

impl TurnGate {
    pub fn observe(&mut self, event: &Event) {
        match event {
            Event::BackgroundAgents { task_ids } => {
                self.agents = task_ids.clone();
                self.release_at = (self.agents.is_empty() && self.held.is_some())
                    .then(|| Instant::now() + FOLLOW_UP_GRACE);
            }
            // A new turn has its own end, so the one held back no longer matters.
            Event::Init { .. } => {
                self.held = None;
                self.release_at = None;
            }
            Event::PromptTaken => self.untaken = self.untaken.saturating_sub(1),
            _ => {}
        }
    }

    pub fn prompted(&mut self) {
        self.untaken += 1;
    }

    /// Whatever the CLI does with a prompt it hadn't taken when interrupted, the count starts
    /// over, so one it dropped can't keep the thread working.
    pub fn interrupted(&mut self) {
        self.untaken = 0;
    }

    /// Whether the CLI has a prompt it will start a turn on once this one ends.
    pub fn queued(&self) -> bool {
        self.untaken > 0
    }

    /// The event to forward, or nothing if it has to wait. A failed turn never waits.
    pub fn pass(&mut self, event: AgentEvent) -> Option<AgentEvent> {
        let finished = matches!(
            &event,
            AgentEvent::TurnEnd {
                stop_reason: StopReason::EndTurn,
                ..
            }
        );
        if finished && !self.agents.is_empty() {
            self.held = Some(event);
            return None;
        }
        Some(event)
    }

    pub fn running_agents(&self) -> Vec<String> {
        self.agents.clone()
    }

    /// When to give up on a follow-up turn, if the agents have all gone.
    pub fn release_at(&self) -> Option<Instant> {
        self.release_at
    }

    pub fn release(&mut self) -> Option<AgentEvent> {
        self.release_at = None;
        self.held.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn_end(stop_reason: &str) -> AgentEvent {
        AgentEvent::TurnEnd {
            stop_reason: stop_reason.to_string().into(),
            error: None,
        }
    }

    fn agents(ids: &[&str]) -> Event {
        Event::BackgroundAgents {
            task_ids: ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    fn init() -> Event {
        Event::Init {
            session_id: "s".into(),
        }
    }

    #[test]
    fn a_turn_with_no_agents_ends_straight_away() {
        let mut gate = TurnGate::default();
        assert!(gate.pass(turn_end("end_turn")).is_some());
    }

    #[test]
    fn a_failed_turn_ends_even_with_agents_running() {
        let mut gate = TurnGate::default();
        gate.observe(&agents(&["a"]));
        assert!(gate.pass(turn_end("error_during_execution")).is_some());
    }

    #[test]
    fn the_follow_up_turn_replaces_the_held_end() {
        let mut gate = TurnGate::default();
        gate.observe(&agents(&["a"]));
        assert!(gate.pass(turn_end("end_turn")).is_none());
        assert!(gate.release_at().is_none());

        gate.observe(&agents(&[]));
        assert!(gate.release_at().is_some());
        gate.observe(&init());
        assert!(gate.release_at().is_none());
        assert_eq!(gate.release(), None);
        assert!(gate.pass(turn_end("end_turn")).is_some());
    }

    #[test]
    fn an_agent_that_starts_no_follow_up_still_ends_the_turn() {
        let mut gate = TurnGate::default();
        gate.observe(&agents(&["a"]));
        assert!(gate.pass(turn_end("end_turn")).is_none());
        gate.observe(&agents(&[]));
        assert!(gate.release_at().is_some());
        assert_eq!(gate.release(), Some(turn_end("end_turn")));
    }

    #[test]
    fn a_prompt_the_cli_has_not_taken_is_queued_until_it_echoes_it() {
        let mut gate = TurnGate::default();
        gate.prompted();
        gate.observe(&Event::PromptTaken);
        assert!(!gate.queued(), "taken into the turn");

        gate.prompted();
        assert!(gate.queued());
        gate.interrupted();
        assert!(!gate.queued());
        gate.observe(&Event::PromptTaken);
        assert!(!gate.queued());
    }

    #[test]
    fn an_agent_resuming_keeps_the_turn_open() {
        let mut gate = TurnGate::default();
        gate.observe(&agents(&["a"]));
        assert!(gate.pass(turn_end("end_turn")).is_none());
        gate.observe(&agents(&[]));
        gate.observe(&agents(&["a"]));
        assert!(gate.release_at().is_none());
    }
}
