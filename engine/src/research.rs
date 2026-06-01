#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchMilestone {
    pub name: &'static str,
    pub target: &'static str,
    pub measurement: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuningParameter {
    pub name: &'static str,
    pub current: i32,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchRoadmap {
    pub milestones: Vec<ResearchMilestone>,
    pub tuning_parameters: Vec<TuningParameter>,
    pub benchmark_targets: Vec<&'static str>,
}

impl ResearchRoadmap {
    pub fn as_lines(&self) -> Vec<String> {
        let mut lines = vec!["Axiorynth research roadmap".to_string()];
        lines.push("milestones:".to_string());
        for milestone in &self.milestones {
            lines.push(format!(
                "- {} | target: {} | measure: {}",
                milestone.name, milestone.target, milestone.measurement
            ));
        }

        lines.push("tuning parameters:".to_string());
        for param in &self.tuning_parameters {
            lines.push(format!(
                "- {} current {} range {}..{}",
                param.name, param.current, param.min, param.max
            ));
        }

        lines.push("benchmark targets:".to_string());
        for target in &self.benchmark_targets {
            lines.push(format!("- {target}"));
        }
        lines
    }
}

pub fn research_roadmap() -> ResearchRoadmap {
    ResearchRoadmap {
        milestones: vec![
            ResearchMilestone {
                name: "Search selectivity",
                target: "add null-move pruning, LMR, futility pruning",
                measurement: "nodes reduced at equal tactical correctness",
            },
            ResearchMilestone {
                name: "Evaluation tuning",
                target: "SPSA tune handcrafted weights",
                measurement: "self-play Elo gain over baseline",
            },
            ResearchMilestone {
                name: "Opening knowledge",
                target: "book and replay-derived opening preferences",
                measurement: "reduced early-game eval loss",
            },
            ResearchMilestone {
                name: "NNUE prototype",
                target: "feature extraction and small neural evaluator",
                measurement: "Elo gain at fixed node budget",
            },
            ResearchMilestone {
                name: "Engine gauntlet",
                target: "automated matches against fixed opponents",
                measurement: "SPRT or Elo confidence interval",
            },
        ],
        tuning_parameters: vec![
            TuningParameter {
                name: "mobility_weight",
                current: 2,
                min: 0,
                max: 8,
            },
            TuningParameter {
                name: "center_attack_weight",
                current: 8,
                min: 0,
                max: 30,
            },
            TuningParameter {
                name: "isolated_pawn_penalty",
                current: 10,
                min: 0,
                max: 40,
            },
            TuningParameter {
                name: "doubled_pawn_penalty",
                current: 12,
                min: 0,
                max: 40,
            },
            TuningParameter {
                name: "king_ring_attack_penalty",
                current: 8,
                min: 0,
                max: 40,
            },
        ],
        benchmark_targets: vec![
            "perft correctness must remain exact",
            "bench depth 4 must complete without panic",
            "UCI stop must return a legal bestmove",
            "analysis report must not mutate board hash",
            "self-play runner should produce reproducible PGN-like logs",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roadmap_contains_measurable_items() {
        let roadmap = research_roadmap();
        assert!(roadmap.milestones.len() >= 5);
        assert!(roadmap.tuning_parameters.len() >= 5);
        assert!(roadmap.as_lines().iter().any(|line| line.contains("NNUE")));
    }
}
