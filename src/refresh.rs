use std::time::{Duration, Instant};

use crate::model::AnalysisRequest;

pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshReason {
    Launch,
    ControlChanged,
    DatabaseRefresh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefreshJob {
    pub generation: u64,
    pub reason: RefreshReason,
    pub request: AnalysisRequest,
}

#[derive(Debug)]
pub struct RefreshCoordinator {
    debounce: Duration,
    next_generation: u64,
    pending: Option<PendingRefresh>,
    latest_started: u64,
}

#[derive(Debug)]
struct PendingRefresh {
    due_at: Instant,
    job: RefreshJob,
}

impl RefreshCoordinator {
    pub fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            next_generation: 0,
            pending: None,
            latest_started: 0,
        }
    }

    pub fn request(
        &mut self,
        request: AnalysisRequest,
        reason: RefreshReason,
        now: Instant,
    ) -> u64 {
        self.next_generation = self.next_generation.saturating_add(1);
        let generation = self.next_generation;
        self.pending = Some(PendingRefresh {
            due_at: now + self.debounce,
            job: RefreshJob {
                generation,
                reason,
                request,
            },
        });
        generation
    }

    pub fn poll(&mut self, now: Instant) -> Option<RefreshJob> {
        let pending = self.pending.take()?;
        if pending.due_at > now {
            self.pending = Some(pending);
            return None;
        }
        self.latest_started = pending.job.generation;
        Some(pending.job)
    }

    pub fn is_current(&self, generation: u64) -> bool {
        generation == self.next_generation && generation >= self.latest_started
    }

    pub fn latest_generation(&self) -> u64 {
        self.next_generation
    }
}

impl Default for RefreshCoordinator {
    fn default() -> Self {
        Self::new(DEFAULT_DEBOUNCE)
    }
}
