use std::time::Duration;

use tokio::time::Instant;

use super::protocol::Event;
use crate::engines::events::AgentEvent;

/// A finished agent starts its follow-up turn at once, but a stopped one starts none, so its
/// thread can't wait for another `result`.
const FOLLOW_UP_GRACE: Duration = Duration::from_secs(5);

/// Claude reports a turn's `result` while background subagents are still running, and each one
/// that finishes starts a turn of its own. This keeps a turn's end back until they are done, so
/// the thread doesn't read as idle while it is still working.
#[derive(Default)]
pub struct TurnGate {
    agents: Vec<String>,
    held: Option<AgentEvent>,
    release_at: Option<Instant>,
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
            _ => {}
        }
    }

    /// The event to forward, or nothing if it has to wait. A failed turn never waits.
    pub fn pass(&mut self, event: AgentEvent) -> Option<AgentEvent> {
        let finished = matches!(
            &event,
            AgentEvent::TurnEnd { stop_reason, .. } if stop_reason == "end_turn"
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
            stop_reason: stop_reason.into(),
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
            permission_mode: None,
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
    fn an_agent_resuming_keeps_the_turn_open() {
        let mut gate = TurnGate::default();
        gate.observe(&agents(&["a"]));
        assert!(gate.pass(turn_end("end_turn")).is_none());
        gate.observe(&agents(&[]));
        gate.observe(&agents(&["a"]));
        assert!(gate.release_at().is_none());
    }
}
