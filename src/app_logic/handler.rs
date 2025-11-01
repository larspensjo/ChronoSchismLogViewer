use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::app_logic::ids::{
    CONTROL_ID_LEFT_VIEWER, CONTROL_ID_RIGHT_VIEWER, CONTROL_ID_TIMESTAMP_INPUT,
    MENU_ACTION_OPEN_LEFT, MENU_ACTION_OPEN_RIGHT,
};
use crate::core::{
    ComparableLine, DiffEngineOperations, DiffLine, DiffState, LineContent, TimestampParserError,
    TimestampParserOperations,
};
use commanductui::StyleId;
use commanductui::types::{
    AppEvent, ControlId, MenuActionId, MessageSeverity, PlatformCommand, PlatformEventHandler,
    TreeItemId, UiStateProvider, WindowId,
};
use regex::Regex;

const LOG_FILE_DIALOG_FILTER: &str = concat!(
    "Log Files (*.log; *.txt)\0*.log;*.txt\0",
    "All Files (*.*)\0*.*\0\0"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingFileDialog {
    Left,
    Right,
}

/// Presenter orchestrating file loading and diff requests per [CSV-Core-CompareV1].
pub struct AppLogic {
    diff_engine: Arc<dyn DiffEngineOperations>,
    timestamp_parser: Arc<dyn TimestampParserOperations>,
    left_file_path: Option<PathBuf>,
    right_file_path: Option<PathBuf>,
    timestamp_pattern: String,
    diff_lines: Vec<DiffLine>,
    pending_commands: VecDeque<PlatformCommand>,
    active_window: Option<WindowId>,
    pending_file_dialog: Option<PendingFileDialog>,
    timestamp_pattern_is_valid: bool,
}

impl AppLogic {
    /// Constructs a new presenter instance with injected dependencies per [CSV-Tech-DIV1].
    pub fn new(
        diff_engine: Arc<dyn DiffEngineOperations>,
        timestamp_parser: Arc<dyn TimestampParserOperations>,
    ) -> Self {
        Self {
            diff_engine,
            timestamp_parser,
            left_file_path: None,
            right_file_path: None,
            timestamp_pattern: String::new(),
            diff_lines: Vec::new(),
            pending_commands: VecDeque::new(),
            active_window: None,
            pending_file_dialog: None,
            timestamp_pattern_is_valid: true,
        }
    }

    fn enqueue_command(&mut self, command: PlatformCommand) {
        self.pending_commands.push_back(command);
    }

    fn handle_menu_action(&mut self, action_id: MenuActionId) {
        if action_id == MENU_ACTION_OPEN_LEFT {
            self.request_open_file_dialog(PendingFileDialog::Left);
        } else if action_id == MENU_ACTION_OPEN_RIGHT {
            self.request_open_file_dialog(PendingFileDialog::Right);
        }
    }

    fn request_open_file_dialog(&mut self, dialog: PendingFileDialog) {
        let Some(window_id) = self.active_window else {
            return;
        };

        let title = match dialog {
            PendingFileDialog::Left => "Open Left Log File",
            PendingFileDialog::Right => "Open Right Log File",
        }
        .to_string();

        let initial_dir = self
            .path_for_dialog(dialog)
            .and_then(|path| path.parent().map(Path::to_path_buf));

        self.pending_file_dialog = Some(dialog);
        self.enqueue_command(PlatformCommand::ShowOpenFileDialog {
            window_id,
            title,
            filter_spec: LOG_FILE_DIALOG_FILTER.to_string(),
            initial_dir,
        });
    }

    fn handle_file_dialog_result(&mut self, window_id: WindowId, result: Option<PathBuf>) {
        if Some(window_id) != self.active_window {
            return;
        }

        let Some(dialog) = self.pending_file_dialog.take() else {
            return;
        };

        if let Some(path) = result {
            match dialog {
                PendingFileDialog::Left => self.left_file_path = Some(path),
                PendingFileDialog::Right => self.right_file_path = Some(path),
            }
            self.trigger_diff_if_ready();
        }
    }

    fn handle_timestamp_input_changed(&mut self, control_id: ControlId, text: String) {
        if control_id != CONTROL_ID_TIMESTAMP_INPUT {
            return;
        }

        self.timestamp_pattern = text;
        let is_valid = self.validate_timestamp_pattern();
        if is_valid {
            self.trigger_diff_if_ready();
        }
    }

    fn trigger_diff_if_ready(&mut self) {
        if !self.timestamp_pattern_is_valid {
            return;
        }

        let Some(window_id) = self.active_window else {
            return;
        };

        let (Some(left_path), Some(right_path)) =
            (self.left_file_path.clone(), self.right_file_path.clone())
        else {
            return;
        };

        match self.execute_diff(&left_path, &right_path) {
            Ok(diff_lines) => {
                self.diff_lines = diff_lines.clone();
                self.enqueue_diff_commands(window_id, &diff_lines);
            }
            Err(err) => self.enqueue_error_dialog(window_id, err),
        }
    }

    fn execute_diff(
        &self,
        left_path: &Path,
        right_path: &Path,
    ) -> Result<Vec<DiffLine>, DiffWorkflowError> {
        let left_lines = read_file_lines(left_path).map_err(|source| DiffWorkflowError::Io {
            path: left_path.to_path_buf(),
            source,
        })?;
        let right_lines = read_file_lines(right_path).map_err(|source| DiffWorkflowError::Io {
            path: right_path.to_path_buf(),
            source,
        })?;

        let stripped_left = self
            .timestamp_parser
            .strip_timestamps(&left_lines, &self.timestamp_pattern)
            .map_err(DiffWorkflowError::Timestamp)?;
        let stripped_right = self
            .timestamp_parser
            .strip_timestamps(&right_lines, &self.timestamp_pattern)
            .map_err(DiffWorkflowError::Timestamp)?;
        debug_assert_eq!(left_lines.len(), stripped_left.len());
        debug_assert_eq!(right_lines.len(), stripped_right.len());

        let comparable_left = Self::build_comparable_lines(&left_lines, &stripped_left);
        let comparable_right = Self::build_comparable_lines(&right_lines, &stripped_right);

        let diff_result = self
            .diff_engine
            .compute_diff(&comparable_left, &comparable_right);
        Ok(diff_result.lines().to_vec())
    }

    fn build_comparable_lines(
        original: &[String],
        stripped: &[String],
    ) -> Vec<ComparableLine> {
        debug_assert_eq!(original.len(), stripped.len());
        original
            .iter()
            .zip(stripped.iter())
            .map(|(original_text, comparable_text)| {
                ComparableLine::new(original_text.clone(), comparable_text.clone())
            })
            .collect()
    }

    fn enqueue_diff_commands(&mut self, window_id: WindowId, lines: &[DiffLine]) {
        let (left_text, right_text) = build_viewer_text(lines);
        self.enqueue_command(PlatformCommand::SetViewerContent {
            window_id,
            control_id: CONTROL_ID_LEFT_VIEWER,
            text: left_text,
        });
        self.enqueue_command(PlatformCommand::SetViewerContent {
            window_id,
            control_id: CONTROL_ID_RIGHT_VIEWER,
            text: right_text,
        });
    }

    fn enqueue_error_dialog(&mut self, window_id: WindowId, error: DiffWorkflowError) {
        let message = match &error {
            DiffWorkflowError::Io { path, source } => {
                format!("Failed to read '{}': {}", path.display(), source)
            }
            DiffWorkflowError::Timestamp(TimestampParserError::InvalidPattern {
                pattern,
                message,
            }) => format!(
                "The timestamp pattern '{}' is invalid: {}",
                pattern, message
            ),
            DiffWorkflowError::Timestamp(TimestampParserError::ProcessingFailed { message }) => {
                format!("Failed to strip timestamps: {message}")
            }
        };

        self.enqueue_command(PlatformCommand::ShowMessageBox {
            window_id,
            title: "Diff Failed".to_string(),
            message,
            severity: MessageSeverity::Error,
        });
    }

    fn path_for_dialog(&self, dialog: PendingFileDialog) -> Option<&PathBuf> {
        match dialog {
            PendingFileDialog::Left => self.left_file_path.as_ref(),
            PendingFileDialog::Right => self.right_file_path.as_ref(),
        }
    }

    fn validate_timestamp_pattern(&mut self) -> bool {
        let pattern = self.timestamp_pattern.clone();
        let is_valid = pattern.is_empty() || Regex::new(&pattern).is_ok();

        if is_valid != self.timestamp_pattern_is_valid {
            self.timestamp_pattern_is_valid = is_valid;
            if let Some(window_id) = self.active_window {
                // [CSV-UX-TimestampFeedbackV1] Keep the input styled to reflect regex validity.
                let style_id = if is_valid {
                    StyleId::DefaultInput
                } else {
                    StyleId::DefaultInputError
                };
                self.enqueue_command(PlatformCommand::ApplyStyleToControl {
                    window_id,
                    control_id: CONTROL_ID_TIMESTAMP_INPUT,
                    style_id,
                });
            }
        }

        is_valid
    }
}

impl PlatformEventHandler for AppLogic {
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::MainWindowUISetupComplete { window_id } => {
                self.active_window = Some(window_id);
            }
            AppEvent::MenuActionClicked { action_id } => self.handle_menu_action(action_id),
            AppEvent::FileOpenProfileDialogCompleted { window_id, result } => {
                self.handle_file_dialog_result(window_id, result)
            }
            AppEvent::InputTextChanged {
                control_id, text, ..
            } => self.handle_timestamp_input_changed(control_id, text),
            AppEvent::WindowDestroyed { window_id } => {
                if Some(window_id) == self.active_window {
                    self.active_window = None;
                }
            }
            _ => {}
        }
    }

    fn try_dequeue_command(&mut self) -> Option<PlatformCommand> {
        self.pending_commands.pop_front()
    }
}

impl UiStateProvider for AppLogic {
    fn is_tree_item_new(&self, _window_id: WindowId, _item_id: TreeItemId) -> bool {
        false
    }
}

#[derive(Debug)]
enum DiffWorkflowError {
    Io { path: PathBuf, source: io::Error },
    Timestamp(TimestampParserError),
}

fn read_file_lines(path: &Path) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    BufReader::new(file).lines().collect()
}

fn build_viewer_text(lines: &[DiffLine]) -> (String, String) {
    let mut left_buffer = Vec::with_capacity(lines.len());
    let mut right_buffer = Vec::with_capacity(lines.len());

    for line in lines {
        let state = line.state();
        let left = format_line_for_side(state, line.left());
        let right = format_line_for_side(state, line.right());
        left_buffer.push(left);
        right_buffer.push(right);
    }

    (left_buffer.join("\r\n"), right_buffer.join("\r\n"))
}

fn format_line_for_side(state: DiffState, content: Option<&LineContent>) -> String {
    let (prefix, text) = match (state, content) {
        (DiffState::Added, None) => ("+", String::new()),
        (DiffState::Deleted, None) => ("-", String::new()),
        (DiffState::Moved, None) => ("↔", String::new()),
        (_, None) => (" ", String::new()),
        (DiffState::Added, Some(line)) => ("+", line.text().to_string()),
        (DiffState::Deleted, Some(line)) => ("-", line.text().to_string()),
        (DiffState::Moved, Some(line)) => ("↔", line.text().to_string()),
        (DiffState::Unchanged, Some(line)) => (" ", line.text().to_string()),
    };

    format!("{prefix} {text}")
}
