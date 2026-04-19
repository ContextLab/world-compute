//! Churn simulator — statistical model for node churn resilience (T113-T115).

use rand::Rng;

/// Result of a single simulation round.
#[derive(Debug)]
#[allow(dead_code)] // exposed for test inspection; not all fields read in every test
pub struct SimulationResult {
    pub nodes_alive: usize,
    pub nodes_churned: usize,
    pub jobs_completed_this_round: usize,
    pub jobs_failed_this_round: usize,
}

/// Simulates node churn and job completion in a federated compute cluster.
///
/// Each round, `churn_rate` fraction of nodes go offline, remaining nodes
/// process jobs, some jobs fail and need rescheduling.
pub struct ChurnSimulator {
    pub node_count: usize,
    pub churn_rate: f64,
    pub job_count: usize,
    pub completed: usize,
    pub failed: usize,
    remaining_jobs: usize,
}

impl ChurnSimulator {
    pub fn new(nodes: usize, churn_rate: f64) -> Self {
        Self {
            node_count: nodes,
            churn_rate: churn_rate.clamp(0.0, 1.0),
            job_count: 0,
            completed: 0,
            failed: 0,
            remaining_jobs: 0,
        }
    }

    /// Submit jobs to the simulator.
    pub fn submit_jobs(&mut self, count: usize) {
        self.job_count += count;
        self.remaining_jobs += count;
    }

    /// Simulate one round of computation with churn.
    ///
    /// In each round:
    /// 1. `churn_rate` fraction of nodes go offline
    /// 2. Remaining nodes each process up to 1 job
    /// 3. Jobs on churned nodes fail and are rescheduled
    pub fn simulate_round(&mut self) -> SimulationResult {
        let mut rng = rand::thread_rng();

        // Determine which nodes are alive this round
        let mut alive = 0usize;
        let mut churned = 0usize;
        for _ in 0..self.node_count {
            if rng.gen::<f64>() >= self.churn_rate {
                alive += 1;
            } else {
                churned += 1;
            }
        }

        // Assign jobs to alive nodes (1 job per node max)
        let assignable = alive.min(self.remaining_jobs);
        let mut completed_this_round = 0usize;
        let mut failed_this_round = 0usize;

        for _ in 0..assignable {
            // Each assigned job has a small chance of failing even on alive nodes
            if rng.gen::<f64>() > 0.05 {
                completed_this_round += 1;
            } else {
                failed_this_round += 1;
            }
        }

        self.completed += completed_this_round;
        self.failed += failed_this_round;
        // Failed jobs go back to the queue for rescheduling
        self.remaining_jobs -= completed_this_round;

        SimulationResult {
            nodes_alive: alive,
            nodes_churned: churned,
            jobs_completed_this_round: completed_this_round,
            jobs_failed_this_round: failed_this_round,
        }
    }

    /// Completion rate = completed / total submitted jobs.
    pub fn completion_rate(&self) -> f64 {
        if self.job_count == 0 {
            return 0.0;
        }
        self.completed as f64 / self.job_count as f64
    }
}

#[test]
fn churn_simulator_20_nodes_30pct_100_jobs() {
    let mut sim = ChurnSimulator::new(20, 0.30);
    sim.submit_jobs(100);

    // Run enough rounds for jobs to complete
    for _ in 0..50 {
        sim.simulate_round();
    }

    let rate = sim.completion_rate();
    assert!(
        rate >= 0.80,
        "20 nodes, 30% churn, 100 jobs over 50 rounds should achieve >= 80% completion, got {:.2}%",
        rate * 100.0
    );
}

#[test]
fn zero_churn_completes_all_jobs() {
    let mut sim = ChurnSimulator::new(10, 0.0);
    sim.submit_jobs(20);

    for _ in 0..10 {
        sim.simulate_round();
    }

    // With 0% churn and 10 nodes over 10 rounds, should complete nearly all
    assert!(sim.completion_rate() > 0.90, "Zero churn should complete nearly all jobs");
}

#[test]
fn high_churn_still_makes_progress() {
    let mut sim = ChurnSimulator::new(50, 0.70);
    sim.submit_jobs(50);

    for _ in 0..100 {
        sim.simulate_round();
    }

    // Even 70% churn with 50 nodes over 100 rounds should complete some jobs
    assert!(sim.completed > 0, "Even high churn should complete some jobs");
}
