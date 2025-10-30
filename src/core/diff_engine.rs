use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffState {
    Added,
    Deleted,
    Unchanged,
    Moved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineContent {
    pub line_number: usize,
    pub text: String,
}

impl LineContent {
    pub fn new(line_number: usize, text: impl Into<String>) -> Self {
        Self {
            line_number,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub state: DiffState,
    pub left: Option<LineContent>,
    pub right: Option<LineContent>,
    pub moved_from: Option<usize>,
    pub moved_to: Option<usize>,
}

impl DiffLine {
    pub fn new(state: DiffState, left: Option<LineContent>, right: Option<LineContent>) -> Self {
        Self {
            state,
            left,
            right,
            moved_from: None,
            moved_to: None,
        }
    }

    pub fn with_movement(mut self, moved_from: Option<usize>, moved_to: Option<usize>) -> Self {
        self.moved_from = moved_from;
        self.moved_to = moved_to;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffStatistics {
    pub additions: usize,
    pub deletions: usize,
    pub moves: usize,
    pub unchanged: usize,
}

impl DiffStatistics {
    pub fn from_lines(lines: &[DiffLine]) -> Self {
        let mut stats = Self::default();
        for line in lines {
            match line.state {
                DiffState::Added => stats.additions += 1,
                DiffState::Deleted => stats.deletions += 1,
                DiffState::Unchanged => stats.unchanged += 1,
                DiffState::Moved => stats.moves += 1,
            }
        }

        stats
    }

    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions + self.moves
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedBlock {
    pub source_start: usize,
    pub source_end: usize,
    pub destination_start: usize,
    pub destination_end: usize,
}

impl MovedBlock {
    pub fn new(
        source_start: usize,
        source_end: usize,
        destination_start: usize,
        destination_end: usize,
    ) -> Self {
        Self {
            source_start,
            source_end,
            destination_start,
            destination_end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub statistics: DiffStatistics,
    pub moved_blocks: Vec<MovedBlock>,
}

impl DiffResult {
    pub fn new(lines: Vec<DiffLine>) -> Self {
        let statistics = DiffStatistics::from_lines(&lines);
        Self {
            lines,
            statistics,
            moved_blocks: Vec::new(),
        }
    }

    pub fn with_moved_blocks(lines: Vec<DiffLine>, moved_blocks: Vec<MovedBlock>) -> Self {
        let statistics = DiffStatistics::from_lines(&lines);
        Self {
            lines,
            statistics,
            moved_blocks,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

pub struct HeckelDiffEngine;

impl HeckelDiffEngine {
    pub fn new() -> Self {
        Self
    }

    fn build_symbol_table<'a>(
        lines_a: &'a [String],
        lines_b: &'a [String],
    ) -> HashMap<&'a str, (usize, usize)> {
        let mut table: HashMap<&'a str, (usize, usize)> = HashMap::new();

        for line in lines_a {
            let entry = table.entry(line.as_str()).or_insert((0, 0));
            entry.0 += 1;
        }

        for line in lines_b {
            let entry = table.entry(line.as_str()).or_insert((0, 0));
            entry.1 += 1;
        }

        table
    }

    fn link_unique_anchors<'a>(
        lines_a: &'a [String],
        lines_b: &'a [String],
        table: &HashMap<&'a str, (usize, usize)>,
    ) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
        let mut oa: Vec<Option<usize>> = vec![None; lines_a.len()];
        let mut na: Vec<Option<usize>> = vec![None; lines_b.len()];

        for (i, line) in lines_a.iter().enumerate() {
            if let Some((1, 1)) = table.get(line.as_str()) {
                if let Some(j) = lines_b.iter().position(|l| l == line) {
                    oa[i] = Some(j);
                    na[j] = Some(i);
                }
            }
        }

        (oa, na)
    }
}

pub trait DiffEngineOperations: Send + Sync {
    fn compute_diff(&self, lines_a: &[String], lines_b: &[String]) -> DiffResult;
}

impl DiffEngineOperations for HeckelDiffEngine {
    fn compute_diff(&self, lines_a: &[String], lines_b: &[String]) -> DiffResult {
        let table = Self::build_symbol_table(lines_a, lines_b);
        let (_oa, _na) = Self::link_unique_anchors(lines_a, lines_b, &table);

        DiffResult::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_symbol_table() {
        let lines_a = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        let lines_b = vec!["c".to_string(), "b".to_string()];

        let table = HeckelDiffEngine::build_symbol_table(&lines_a, &lines_b);

        assert_eq!(table.get("a"), Some(&(2, 0)));
        assert_eq!(table.get("b"), Some(&(1, 1)));
        assert_eq!(table.get("c"), Some(&(0, 1)));
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn test_unique_anchor_linking() {
        let lines_a = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        let lines_b = vec![
            "delta".to_string(),
            "beta".to_string(),
            "epsilon".to_string(),
        ];

        let table = HeckelDiffEngine::build_symbol_table(&lines_a, &lines_b);
        let (oa, na) = HeckelDiffEngine::link_unique_anchors(&lines_a, &lines_b, &table);

        assert_eq!(oa, vec![None, Some(1), None]);
        assert_eq!(na, vec![None, Some(1), None]);
    }
}
