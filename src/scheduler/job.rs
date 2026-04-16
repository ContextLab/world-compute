//! Job, Workflow, Task, Replica state machines per data-model §3.6-3.9 (T055-T058).

use crate::types::{Cid, NcuAmount, PeerIdStr, Timestamp};
use serde::{Deserialize, Serialize};

/// Workflow state per data-model §3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowState {
    Pending,
    Running,
    Checkpointed,
    Completed,
    Failed,
}

/// Job state per data-model §3.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobState {
    Queued,
    Dispatching,
    Running,
    Verifying,
    Completed,
    Checkpointed,
    Failed,
    Cancelled,
}

/// Task state per data-model §3.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    Ready,
    Dispatched,
    Running,
    Checkpointing,
    Verifying,
    Accepted,
    Failed,
}

/// Replica state per data-model §3.9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplicaState {
    Leased,
    Running,
    Checkpointing,
    Completed,
    Failed,
    Preempted,
    Expired,
}

/// A live job instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub manifest_cid: Cid,
    pub state: JobState,
    pub submitter_id: String,
    pub priority_score: f64,
    pub ncu_reserved: NcuAmount,
    pub created_at: Timestamp,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
}

/// A live task instance (atomic scheduling unit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub job_id: String,
    pub state: TaskState,
    pub workload_cid: Cid,
    pub replica_count: u32,
    pub checkpoint_cid: Option<Cid>,
    pub checkpoint_sequence: u32,
    pub created_at: Timestamp,
}

/// A single replica execution instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    pub replica_id: String,
    pub task_id: String,
    pub node_id: PeerIdStr,
    pub state: ReplicaState,
    pub result_cid: Option<Cid>,
    pub execution_ms: Option<u64>,
    pub lease_expires_at: Timestamp,
}

impl Job {
    /// Attempt a state transition. Returns Err if the transition is invalid.
    pub fn transition(&mut self, new_state: JobState) -> Result<(), String> {
        let valid = matches!(
            (self.state, new_state),
            (JobState::Queued, JobState::Dispatching)
                | (JobState::Dispatching, JobState::Running)
                | (JobState::Running, JobState::Verifying)
                | (JobState::Running, JobState::Checkpointed)
                | (JobState::Running, JobState::Failed)
                | (JobState::Verifying, JobState::Completed)
                | (JobState::Verifying, JobState::Failed)
                | (JobState::Checkpointed, JobState::Dispatching)
                | (JobState::Queued, JobState::Cancelled)
                | (JobState::Dispatching, JobState::Cancelled)
                | (JobState::Running, JobState::Cancelled)
        );
        if valid {
            self.state = new_state;
            Ok(())
        } else {
            Err(format!("Invalid transition: {:?} → {:?}", self.state, new_state))
        }
    }
}

impl Task {
    /// Attempt a state transition.
    pub fn transition(&mut self, new_state: TaskState) -> Result<(), String> {
        let valid = matches!(
            (self.state, new_state),
            (TaskState::Ready, TaskState::Dispatched)
                | (TaskState::Dispatched, TaskState::Running)
                | (TaskState::Running, TaskState::Checkpointing)
                | (TaskState::Running, TaskState::Verifying)
                | (TaskState::Running, TaskState::Failed)
                | (TaskState::Checkpointing, TaskState::Running)
                | (TaskState::Verifying, TaskState::Accepted)
                | (TaskState::Verifying, TaskState::Failed)
        );
        if valid {
            self.state = new_state;
            Ok(())
        } else {
            Err(format!("Invalid transition: {:?} → {:?}", self.state, new_state))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;

    fn test_job() -> Job {
        Job {
            job_id: "job-001".into(),
            manifest_cid: compute_cid(b"test manifest").unwrap(),
            state: JobState::Queued,
            submitter_id: "sub-001".into(),
            priority_score: 0.5,
            ncu_reserved: NcuAmount::from_ncu(1.0),
            created_at: Timestamp::now(),
            started_at: None,
            completed_at: None,
        }
    }

    fn test_task() -> Task {
        Task {
            task_id: "task-001".into(),
            job_id: "job-001".into(),
            state: TaskState::Ready,
            workload_cid: compute_cid(b"test workload").unwrap(),
            replica_count: 3,
            checkpoint_cid: None,
            checkpoint_sequence: 0,
            created_at: Timestamp::now(),
        }
    }

    #[test]
    fn job_valid_transitions() {
        let mut job = test_job();
        assert!(job.transition(JobState::Dispatching).is_ok());
        assert!(job.transition(JobState::Running).is_ok());
        assert!(job.transition(JobState::Verifying).is_ok());
        assert!(job.transition(JobState::Completed).is_ok());
    }

    #[test]
    fn job_invalid_transition_rejected() {
        let mut job = test_job();
        assert!(job.transition(JobState::Completed).is_err());
    }

    #[test]
    fn job_cancel_from_queued() {
        let mut job = test_job();
        assert!(job.transition(JobState::Cancelled).is_ok());
    }

    #[test]
    fn task_valid_transitions() {
        let mut task = test_task();
        assert!(task.transition(TaskState::Dispatched).is_ok());
        assert!(task.transition(TaskState::Running).is_ok());
        assert!(task.transition(TaskState::Verifying).is_ok());
        assert!(task.transition(TaskState::Accepted).is_ok());
    }

    #[test]
    fn task_checkpoint_cycle() {
        let mut task = test_task();
        assert!(task.transition(TaskState::Dispatched).is_ok());
        assert!(task.transition(TaskState::Running).is_ok());
        assert!(task.transition(TaskState::Checkpointing).is_ok());
        assert!(task.transition(TaskState::Running).is_ok()); // resume after checkpoint
    }

    #[test]
    fn task_invalid_transition_rejected() {
        let mut task = test_task();
        assert!(task.transition(TaskState::Accepted).is_err());
    }
}
