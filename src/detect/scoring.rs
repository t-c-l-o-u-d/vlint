// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use crate::catalog::linter::LinterId;

/// Per-file scorer implementing weighted consensus with tiebreakers.
pub struct FileScore {
    scores: HashMap<LinterId, u32>,
    max_weights: HashMap<LinterId, u32>,
}

impl Default for FileScore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileScore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            max_weights: HashMap::new(),
        }
    }

    /// Get a summary of all scores for verbose output.
    #[must_use]
    pub fn summary(&self) -> Vec<(LinterId, u32)> {
        let mut scores: Vec<_> = self.scores.iter().map(|(&id, &s)| (id, s)).collect();
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        scores
    }

    pub fn vote(&mut self, linter: LinterId, weight: u32) {
        *self.scores.entry(linter).or_insert(0) += weight;
        let max = self.max_weights.entry(linter).or_insert(0);
        if weight > *max {
            *max = weight;
        }
    }

    /// Determine the winning linter using tiebreaker rules:
    /// 1. Highest total score wins.
    /// 2. Tie: Skip always loses to a real linter.
    /// 3. Tie: Highest individual vote weight wins.
    #[must_use]
    pub fn winner(&self) -> Option<LinterId> {
        if self.scores.is_empty() {
            return None;
        }

        let max_score = self.scores.values().copied().max().unwrap_or(0);
        let candidates: Vec<LinterId> = self
            .scores
            .iter()
            .filter(|(_, s)| **s == max_score)
            .map(|(&id, _)| id)
            .collect();

        if candidates.len() == 1 {
            return Some(candidates[0]);
        }

        // Tiebreaker 1: Skip loses to any real linter.
        let non_skip: Vec<LinterId> = candidates
            .iter()
            .filter(|&&id| id != LinterId::Skip)
            .copied()
            .collect();

        if non_skip.len() == 1 {
            return Some(non_skip[0]);
        }

        let remaining = if non_skip.is_empty() {
            &candidates
        } else {
            &non_skip
        };

        // Tiebreaker 2: Highest max individual vote weight.
        let mut best = remaining[0];
        let mut best_max_w = self.max_weights.get(&best).copied().unwrap_or(0);
        for &id in &remaining[1..] {
            let w = self.max_weights.get(&id).copied().unwrap_or(0);
            if w > best_max_w {
                best = id;
                best_max_w = w;
            }
        }

        Some(best)
    }
}
