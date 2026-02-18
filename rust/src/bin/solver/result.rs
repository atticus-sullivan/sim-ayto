use serde::{Serialize, Deserialize};

use ayto::constraint::eval_types::EvalEvent;

/// Collects the results of one entire simulation (playing the game once)
#[derive(Serialize, Deserialize)]
pub(crate) struct SimulationResult {
    sim_id: usize,
    seed: u64,
    stats: Vec<EvalEvent>,
    iterations_count: usize,
    duration_ms: u128,
}

impl SimulationResult {
    pub(crate) fn new(
        sim_id: usize,
        seed: u64,
        stats: Vec<EvalEvent>,
        iterations_count: usize,
        duration_ms: u128,
    ) -> Self {
        Self {
            sim_id,
            seed,
            stats,
            iterations_count,
            duration_ms,
        }
    }

    pub(crate) fn identifier(&self) -> usize {
        self.sim_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------
    // constructor wiring
    // ------------------------------------------------------------

    #[test]
    fn new_sets_all_fields_correctly() {
        let stats = Vec::<EvalEvent>::new();

        let result = SimulationResult::new(
            42,
            1337,
            stats.clone(),
            10,
            250,
        );

        assert_eq!(result.sim_id, 42);
        assert_eq!(result.seed, 1337);
        assert_eq!(result.iterations_count, 10);
        assert_eq!(result.duration_ms, 250);
        assert_eq!(result.stats.len(), 0);
    }

    // ------------------------------------------------------------
    // identifier
    // ------------------------------------------------------------

    #[test]
    fn identifier_returns_sim_id() {
        let result = SimulationResult::new(
            7,
            999,
            vec![],
            3,
            100,
        );

        assert_eq!(result.identifier(), 7);
    }

    // ------------------------------------------------------------
    // serialization
    // ------------------------------------------------------------

    #[test]
    fn serializes_to_json() {
        let result = SimulationResult::new(
            1,
            2,
            vec![],
            3,
            4,
        );

        let json = serde_json::to_string(&result).expect("serialization failed");

        // Ensure essential fields are present
        assert!(json.contains("\"sim_id\":1"));
        assert!(json.contains("\"seed\":2"));
        assert!(json.contains("\"iterations_count\":3"));
        assert!(json.contains("\"duration_ms\":4"));
    }

    // ------------------------------------------------------------
    // edge cases
    // ------------------------------------------------------------

    #[test]
    fn handles_zero_values() {
        let result = SimulationResult::new(
            0,
            0,
            vec![],
            0,
            0,
        );

        assert_eq!(result.identifier(), 0);
        assert_eq!(result.iterations_count, 0);
        assert_eq!(result.duration_ms, 0);
    }
}
