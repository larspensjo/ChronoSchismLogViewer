use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct ComparableLine {
    pub original_text: String,
    pub comparable_text: String,
}

impl ComparableLine {
    pub fn new(original_text: impl Into<String>, comparable_text: impl Into<String>) -> Self {
        Self {
            original_text: original_text.into(),
            comparable_text: comparable_text.into(),
        }
    }
}

impl PartialEq for ComparableLine {
    fn eq(&self, other: &Self) -> bool {
        self.comparable_text == other.comparable_text
    }
}

impl Eq for ComparableLine {}

impl Hash for ComparableLine {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.comparable_text.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffState {
    Added,
    Deleted,
    Unchanged,
    Moved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineContent {
    line_number: usize,
    text: String,
}

impl LineContent {
    pub fn new(line_number: usize, text: impl Into<String>) -> Self {
        Self {
            line_number,
            text: text.into(),
        }
    }

    // Accessors uphold [CSV-Tech-EncapsulationV1] & [CSV-Tech-TraceabilityV1].
    pub fn line_number(&self) -> usize {
        self.line_number
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    state: DiffState,
    left: Option<LineContent>,
    right: Option<LineContent>,
    moved_from: Option<usize>,
    moved_to: Option<usize>,
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

    pub fn state(&self) -> DiffState {
        self.state.clone()
    }

    pub fn left(&self) -> Option<&LineContent> {
        self.left.as_ref()
    }

    pub fn right(&self) -> Option<&LineContent> {
        self.right.as_ref()
    }

    pub fn moved_from(&self) -> Option<usize> {
        self.moved_from
    }

    pub fn moved_to(&self) -> Option<usize> {
        self.moved_to
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffStatistics {
    additions: usize,
    deletions: usize,
    moves: usize,
    unchanged: usize,
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

    pub fn additions(&self) -> usize {
        self.additions
    }

    pub fn deletions(&self) -> usize {
        self.deletions
    }

    pub fn moves(&self) -> usize {
        self.moves
    }

    pub fn unchanged(&self) -> usize {
        self.unchanged
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedBlock {
    source_start: usize,
    source_end: usize,
    destination_start: usize,
    destination_end: usize,
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

    pub fn source_start(&self) -> usize {
        self.source_start
    }

    pub fn source_end(&self) -> usize {
        self.source_end
    }

    pub fn destination_start(&self) -> usize {
        self.destination_start
    }

    pub fn destination_end(&self) -> usize {
        self.destination_end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    lines: Vec<DiffLine>,
    statistics: DiffStatistics,
    moved_blocks: Vec<MovedBlock>,
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

    pub fn lines(&self) -> &[DiffLine] {
        &self.lines
    }

    pub fn statistics(&self) -> &DiffStatistics {
        &self.statistics
    }

    pub fn moved_blocks(&self) -> &[MovedBlock] {
        &self.moved_blocks
    }
}

pub struct HeckelDiffEngine;

impl HeckelDiffEngine {
    pub fn new() -> Self {
        Self
    }

    fn build_symbol_table<'a>(
        lines_a: &'a [ComparableLine],
        lines_b: &'a [ComparableLine],
    ) -> HashMap<&'a ComparableLine, (usize, usize)> {
        let mut table: HashMap<&'a ComparableLine, (usize, usize)> = HashMap::new();

        for line in lines_a {
            let entry = table.entry(line).or_insert((0, 0));
            entry.0 += 1;
        }

        for line in lines_b {
            let entry = table.entry(line).or_insert((0, 0));
            entry.1 += 1;
        }

        table
    }

    fn link_unique_anchors<'a>(
        lines_a: &'a [ComparableLine],
        lines_b: &'a [ComparableLine],
        table: &HashMap<&'a ComparableLine, (usize, usize)>,
    ) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
        let mut oa: Vec<Option<usize>> = vec![None; lines_a.len()];
        let mut na: Vec<Option<usize>> = vec![None; lines_b.len()];

        for (i, line) in lines_a.iter().enumerate() {
            if let Some((1, 1)) = table.get(line) {
                if let Some(j) = lines_b.iter().position(|l| l == line) {
                    oa[i] = Some(j);
                    na[j] = Some(i);
                }
            }
        }

        (oa, na)
    }

    fn link_non_unique_matches<'a>(
        lines_a: &'a [ComparableLine],
        lines_b: &'a [ComparableLine],
        table: &HashMap<&'a ComparableLine, (usize, usize)>,
        oa: &mut [Option<usize>],
        na: &mut [Option<usize>],
    ) {
        for (i, line_a) in lines_a.iter().enumerate() {
            if oa[i].is_some() {
                continue;
            }

            if let Some((count_a, count_b)) = table.get(line_a) {
                if *count_a == 0 || *count_b == 0 {
                    continue;
                }

                for (j, line_b) in lines_b.iter().enumerate() {
                    if na[j].is_none() && line_a == line_b {
                        oa[i] = Some(j);
                        na[j] = Some(i);
                        break;
                    }
                }
            }
        }
    }

    fn build_diff_lines(
        lines_a: &[ComparableLine],
        lines_b: &[ComparableLine],
        oa: &[Option<usize>],
        na: &[Option<usize>],
    ) -> (Vec<DiffLine>, Vec<MovedBlock>) {
        let mut result_lines: Vec<DiffLine> = Vec::new();
        let mut matched_info: Vec<(usize, usize, usize)> = Vec::new();

        let mut processed_old = vec![false; lines_a.len()];
        let mut i_ptr: usize = 0;

        for j in 0..lines_b.len() {
            let matched_old = na
                .get(j)
                .copied()
                .flatten()
                .filter(|&i| oa.get(i).copied().flatten() == Some(j) && !processed_old[i]);

            if let Some(i_match) = matched_old {
                // Emit deletions for old lines that occur before the matched index
                while i_ptr < i_match {
                    if processed_old[i_ptr] {
                        i_ptr += 1;
                        continue;
                    }

                    match oa[i_ptr] {
                        Some(mapped_j) if mapped_j >= j => break,
                        Some(mapped_j) if mapped_j < j => {
                            processed_old[i_ptr] = true;
                            i_ptr += 1;
                        }
                        _ => {
                            result_lines.push(DiffLine::new(
                                DiffState::Deleted,
                                Some(LineContent::new(
                                    i_ptr + 1,
                                    lines_a[i_ptr].original_text.clone(),
                                )),
                                None,
                            ));
                            processed_old[i_ptr] = true;
                            i_ptr += 1;
                        }
                    }
                }

                let line_index = result_lines.len();
                result_lines.push(DiffLine::new(
                    DiffState::Unchanged,
                    Some(LineContent::new(
                        i_match + 1,
                        lines_a[i_match].original_text.clone(),
                    )),
                    Some(LineContent::new(j + 1, lines_b[j].original_text.clone())),
                ));
                matched_info.push((line_index, i_match, j));

                processed_old[i_match] = true;
                if i_ptr == i_match {
                    i_ptr += 1;
                }

                while i_ptr < lines_a.len() && processed_old[i_ptr] {
                    i_ptr += 1;
                }
            } else {
                result_lines.push(DiffLine::new(
                    DiffState::Added,
                    None,
                    Some(LineContent::new(j + 1, lines_b[j].original_text.clone())),
                ));
            }
        }

        // Emit deletions for any remaining old lines
        while i_ptr < lines_a.len() {
            if processed_old[i_ptr] {
                i_ptr += 1;
                continue;
            }

            result_lines.push(DiffLine::new(
                DiffState::Deleted,
                Some(LineContent::new(
                    i_ptr + 1,
                    lines_a[i_ptr].original_text.clone(),
                )),
                None,
            ));
            processed_old[i_ptr] = true;
            i_ptr += 1;
        }

        let moved_blocks = Self::classify_matched_lines(&mut result_lines, &matched_info);

        (result_lines, moved_blocks)
    }

    fn classify_matched_lines(
        lines: &mut [DiffLine],
        matched_info: &[(usize, usize, usize)],
    ) -> Vec<MovedBlock> {
        if matched_info.is_empty() {
            return Vec::new();
        }

        let sequence: Vec<usize> = matched_info
            .iter()
            .map(|(_, old_idx, _)| *old_idx)
            .collect();
        let lis_positions = Self::longest_increasing_subsequence_indices(&sequence);

        let mut is_in_lis = vec![false; matched_info.len()];
        for idx in lis_positions {
            if let Some(flag) = is_in_lis.get_mut(idx) {
                *flag = true;
            }
        }

        let mut moved_blocks = Vec::new();
        let mut current_block: Option<(usize, usize, usize, usize)> = None;

        for (idx, (line_idx, old_idx, new_idx)) in matched_info.iter().enumerate() {
            let line = &mut lines[*line_idx];
            if is_in_lis[idx] {
                line.state = DiffState::Unchanged;
                line.moved_from = None;
                line.moved_to = None;

                if let Some((source_start, source_end, dest_start, dest_end)) = current_block.take()
                {
                    moved_blocks.push(MovedBlock::new(
                        source_start,
                        source_end,
                        dest_start,
                        dest_end,
                    ));
                }
            } else {
                line.state = DiffState::Moved;
                line.moved_from = Some(old_idx + 1);
                line.moved_to = Some(new_idx + 1);

                let source_line = old_idx + 1;
                let dest_line = new_idx + 1;

                match current_block {
                    Some((source_start, _, dest_start, _)) => {
                        current_block = Some((source_start, source_line, dest_start, dest_line));
                    }
                    None => {
                        current_block = Some((source_line, source_line, dest_line, dest_line));
                    }
                }
            }
        }

        if let Some((source_start, source_end, dest_start, dest_end)) = current_block {
            moved_blocks.push(MovedBlock::new(
                source_start,
                source_end,
                dest_start,
                dest_end,
            ));
        }

        moved_blocks
    }

    fn longest_increasing_subsequence_indices(sequence: &[usize]) -> Vec<usize> {
        if sequence.is_empty() {
            return Vec::new();
        }

        let n = sequence.len();
        let mut tail_indices = vec![0usize; n];
        let mut predecessors: Vec<Option<usize>> = vec![None; n];
        let mut length = 0usize;

        for (i, &value) in sequence.iter().enumerate() {
            let mut left = 0usize;
            let mut right = length;

            while left < right {
                let mid = (left + right) / 2;
                if sequence[tail_indices[mid]] < value {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }

            if left > 0 {
                predecessors[i] = Some(tail_indices[left - 1]);
            }

            tail_indices[left] = i;
            if left + 1 > length {
                length = left + 1;
            }
        }

        if length == 0 {
            return Vec::new();
        }

        let mut lis_indices = Vec::with_capacity(length);
        let mut k = tail_indices[length - 1];
        loop {
            lis_indices.push(k);
            if let Some(prev) = predecessors[k] {
                k = prev;
            } else {
                break;
            }
        }
        lis_indices.reverse();
        lis_indices
    }
}

pub trait DiffEngineOperations: Send + Sync {
    fn compute_diff(&self, lines_a: &[ComparableLine], lines_b: &[ComparableLine]) -> DiffResult;
}

impl DiffEngineOperations for HeckelDiffEngine {
    fn compute_diff(&self, lines_a: &[ComparableLine], lines_b: &[ComparableLine]) -> DiffResult {
        let table = Self::build_symbol_table(lines_a, lines_b);
        let (mut oa, mut na) = Self::link_unique_anchors(lines_a, lines_b, &table);
        Self::link_non_unique_matches(lines_a, lines_b, &table, &mut oa, &mut na);
        let (lines, moved_blocks) = Self::build_diff_lines(lines_a, lines_b, &oa, &na);

        DiffResult::with_moved_blocks(lines, moved_blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(text: &str) -> ComparableLine {
        ComparableLine::new(text, text)
    }

    fn comparable(original: &str, stripped: &str) -> ComparableLine {
        ComparableLine::new(original, stripped)
    }

    #[test]
    fn comparable_line_equality_ignores_original_text() {
        let line_a = comparable("[A] entry", "entry");
        let line_b = comparable("[B] entry", "entry");
        let line_c = comparable("[C] other", "other");

        assert_eq!(line_a, line_b, "Comparable text should drive equality");
        assert_ne!(line_a, line_c, "Different comparable text must not be equal");
    }

    #[test]
    fn test_build_symbol_table() {
        let lines_a = vec![same("a"), same("b"), same("a")];
        let lines_b = vec![same("c"), same("b")];

        let table = HeckelDiffEngine::build_symbol_table(&lines_a, &lines_b);

        assert_eq!(table.get(&lines_a[0]), Some(&(2, 0)));
        assert_eq!(table.get(&lines_a[1]), Some(&(1, 1)));
        assert_eq!(table.get(&lines_b[0]), Some(&(0, 1)));
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn test_unique_anchor_linking() {
        let lines_a = vec![same("alpha"), same("beta"), same("gamma")];
        let lines_b = vec![same("delta"), same("beta"), same("epsilon")];

        let table = HeckelDiffEngine::build_symbol_table(&lines_a, &lines_b);
        let (oa, na) = HeckelDiffEngine::link_unique_anchors(&lines_a, &lines_b, &table);

        assert_eq!(oa, vec![None, Some(1), None]);
        assert_eq!(na, vec![None, Some(1), None]);
    }

    #[test]
    fn test_non_unique_matches_are_linked() {
        let lines_a = vec![same("foo"), same("bar"), same("foo")];
        let lines_b = vec![same("foo"), same("foo"), same("bar")];

        let table = HeckelDiffEngine::build_symbol_table(&lines_a, &lines_b);
        let (mut oa, mut na) = HeckelDiffEngine::link_unique_anchors(&lines_a, &lines_b, &table);
        HeckelDiffEngine::link_non_unique_matches(&lines_a, &lines_b, &table, &mut oa, &mut na);

        assert_eq!(oa, vec![Some(0), Some(2), Some(1)]);
        assert_eq!(na, vec![Some(0), Some(2), Some(1)]);
    }

    #[test]
    fn test_compares_stripped_text_but_displays_original() {
        let engine = HeckelDiffEngine::new();
        let lines_a = vec![
            comparable("[T1] line a", "line a"),
            comparable("[T2] line b", "line b"),
        ];
        let lines_b = vec![
            comparable("[T3] line a", "line a"),
            comparable("[T4] line b", "line b"),
        ];

        let result = engine.compute_diff(&lines_a, &lines_b);

        assert_eq!(result.statistics().unchanged(), 2);
        assert_eq!(result.statistics().total_changes(), 0);
        assert!(result.moved_blocks().is_empty());

        let first = &result.lines()[0];
        assert_eq!(first.state(), DiffState::Unchanged);
        assert_eq!(first.left().unwrap().text(), "[T1] line a");
        assert_eq!(first.right().unwrap().text(), "[T3] line a");

        let second = &result.lines()[1];
        assert_eq!(second.state(), DiffState::Unchanged);
        assert_eq!(second.left().unwrap().text(), "[T2] line b");
        assert_eq!(second.right().unwrap().text(), "[T4] line b");
    }

    #[test]
    fn test_simple_addition() {
        let engine = HeckelDiffEngine::new();
        let lines_a = vec![same("a"), same("c")];
        let lines_b = vec![same("a"), same("b"), same("c")];

        let result = engine.compute_diff(&lines_a, &lines_b);

        assert_eq!(result.statistics().additions(), 1);
        assert_eq!(result.statistics().unchanged(), 2);
        assert_eq!(result.lines()[1].state(), DiffState::Added);
        assert_eq!(result.lines()[1].right().map(|c| c.text()), Some("b"));
    }

    #[test]
    fn test_simple_deletion() {
        let engine = HeckelDiffEngine::new();
        let lines_a = vec![same("a"), same("b"), same("c")];
        let lines_b = vec![same("a"), same("c")];

        let result = engine.compute_diff(&lines_a, &lines_b);

        assert_eq!(result.statistics().deletions(), 1);
        assert_eq!(result.statistics().unchanged(), 2);
        assert_eq!(result.lines()[1].state(), DiffState::Deleted);
        assert_eq!(result.lines()[1].left().map(|c| c.text()), Some("b"));
    }

    #[test]
    fn test_simple_move() {
        let engine = HeckelDiffEngine::new();
        let lines_a = vec![same("a"), same("b"), same("c")];
        let lines_b = vec![same("c"), same("a"), same("b")];

        let result = engine.compute_diff(&lines_a, &lines_b);

        assert_eq!(result.statistics().additions(), 0);
        assert_eq!(result.statistics().deletions(), 0);
        assert!(result.statistics().moves() >= 1);
        assert!(
            result
                .lines()
                .iter()
                .any(|line| line.state() == DiffState::Moved)
        );
        assert!(
            !result.moved_blocks().is_empty(),
            "Expected at least one moved block"
        );
    }
}
