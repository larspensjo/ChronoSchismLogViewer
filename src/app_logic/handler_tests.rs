#[cfg(test)]
mod tests {
    use crate::app_logic::handler::AppLogic;
    use crate::app_logic::ids::{
        CONTROL_ID_LEFT_VIEWER, CONTROL_ID_RIGHT_VIEWER, CONTROL_ID_TIMESTAMP_INPUT,
        MENU_ACTION_OPEN_LEFT, MENU_ACTION_OPEN_RIGHT,
    };
    use crate::core::{
        ComparableLine, DiffEngineOperations, DiffLine, DiffState, LineContent,
        TimestampParserOperations,
    };
    use commanductui::types::{AppEvent, PlatformCommand, WindowId};
    use commanductui::{PlatformEventHandler, StyleId};
    use std::collections::VecDeque;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[derive(Default)]
    struct MockTimestampParser {
        calls: Mutex<Vec<(Vec<String>, String)>>,
        responses: Mutex<VecDeque<Vec<String>>>,
    }

    impl MockTimestampParser {
        fn calls(&self) -> Vec<(Vec<String>, String)> {
            self.calls.lock().unwrap().clone()
        }

        fn with_responses(responses: Vec<Vec<String>>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(VecDeque::from(responses)),
            }
        }
    }

    impl TimestampParserOperations for MockTimestampParser {
        fn strip_timestamps(
            &self,
            lines: &[String],
            pattern: &str,
        ) -> Result<Vec<String>, crate::core::TimestampParserError> {
            let captured_lines = lines.to_vec();
            {
                let mut guard = self.calls.lock().unwrap();
                guard.push((captured_lines.clone(), pattern.to_string()));
            }

            let mut responses = self.responses.lock().unwrap();
            if let Some(stripped) = responses.pop_front() {
                Ok(stripped)
            } else {
                Ok(captured_lines)
            }
        }
    }

    struct MockDiffEngine {
        calls: Mutex<Vec<(Vec<ComparableLine>, Vec<ComparableLine>)>>,
        lines_to_return: Vec<DiffLine>,
    }

    impl MockDiffEngine {
        fn new(lines_to_return: Vec<DiffLine>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                lines_to_return,
            }
        }

        fn calls(&self) -> Vec<(Vec<ComparableLine>, Vec<ComparableLine>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl DiffEngineOperations for MockDiffEngine {
        fn compute_diff(
            &self,
            lines_a: &[ComparableLine],
            lines_b: &[ComparableLine],
        ) -> crate::core::DiffResult {
            let mut guard = self.calls.lock().unwrap();
            guard.push((lines_a.to_vec(), lines_b.to_vec()));
            crate::core::DiffResult::new(self.lines_to_return.clone())
        }
    }

    fn snapshot(lines: &[ComparableLine]) -> Vec<(&str, &str)> {
        lines
            .iter()
            .map(|line| (line.original_text.as_str(), line.comparable_text.as_str()))
            .collect()
    }

    #[test]
    fn diff_workflow_enqueues_viewer_updates() {
        // Arrange
        let diff_lines = vec![
            DiffLine::new(
                DiffState::Unchanged,
                Some(LineContent::new(1, "alpha")),
                Some(LineContent::new(1, "alpha")),
            ),
            DiffLine::new(DiffState::Added, None, Some(LineContent::new(2, "beta"))),
        ];
        let mock_diff_engine = Arc::new(MockDiffEngine::new(diff_lines));
        let mock_timestamp_parser = Arc::new(MockTimestampParser::with_responses(vec![
            vec!["alpha".into(), "beta".into()],
            vec!["alpha".into(), "beta".into()],
        ]));

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser);

        let window_id = WindowId::new(7);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });

        let (_temp_dir, left_path, right_path) = create_test_files();

        // Act: open left file
        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_LEFT,
        });

        let open_left = app_logic
            .try_dequeue_command()
            .expect("expected left file dialog command");
        match open_left {
            PlatformCommand::ShowOpenFileDialog { title, .. } => {
                assert!(
                    title.contains("Left"),
                    "expected left open dialog title, got {title}"
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(left_path.clone()),
        });
        assert!(
            app_logic.try_dequeue_command().is_none(),
            "no diff should run until both files selected"
        );

        // Act: open right file
        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_RIGHT,
        });
        let open_right = app_logic
            .try_dequeue_command()
            .expect("expected right file dialog command");
        match open_right {
            PlatformCommand::ShowOpenFileDialog { title, .. } => {
                assert!(
                    title.contains("Right"),
                    "expected right open dialog title, got {title}"
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(right_path.clone()),
        });

        // Assert: diff results enqueued
        let left_update = app_logic
            .try_dequeue_command()
            .expect("expected left viewer update");
        match left_update {
            PlatformCommand::SetViewerContent {
                control_id,
                text,
                window_id: cmd_window,
            } => {
                assert_eq!(cmd_window, window_id);
                assert_eq!(control_id, CONTROL_ID_LEFT_VIEWER);
                assert_eq!(text, "  alpha\r\n+ ");
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let right_update = app_logic
            .try_dequeue_command()
            .expect("expected right viewer update");
        match right_update {
            PlatformCommand::SetViewerContent {
                control_id,
                text,
                window_id: cmd_window,
            } => {
                assert_eq!(cmd_window, window_id);
                assert_eq!(control_id, CONTROL_ID_RIGHT_VIEWER);
                assert_eq!(text, "  alpha\r\n+ beta");
            }
            other => panic!("unexpected command: {other:?}"),
        }

        assert!(
            app_logic.try_dequeue_command().is_none(),
            "no extra commands expected"
        );

        // Assert: dependencies invoked with expected inputs
        let parser_calls = mock_timestamp_parser.calls();
        assert_eq!(parser_calls.len(), 2);
        assert_eq!(
            parser_calls[0].0,
            vec![String::from("left-alpha"), String::from("left-beta")]
        );
        assert_eq!(
            parser_calls[1].0,
            vec![String::from("right-alpha"), String::from("right-beta")]
        );
        assert!(parser_calls.iter().all(|(_, pattern)| pattern.is_empty()));

        let diff_calls = mock_diff_engine.calls();
        assert_eq!(diff_calls.len(), 1);
        assert_eq!(
            snapshot(&diff_calls[0].0),
            vec![("left-alpha", "alpha"), ("left-beta", "beta")]
        );
        assert_eq!(
            snapshot(&diff_calls[0].1),
            vec![("right-alpha", "alpha"), ("right-beta", "beta")]
        );
    }

    #[test]
    fn invalid_regex_applies_error_style_and_blocks_diff_until_valid() {
        let diff_lines = vec![DiffLine::new(
            DiffState::Unchanged,
            Some(LineContent::new(1, "alpha")),
            Some(LineContent::new(1, "alpha")),
        )];
        let mock_diff_engine = Arc::new(MockDiffEngine::new(diff_lines));
        let mock_timestamp_parser = Arc::new(MockTimestampParser::default());

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser);

        let window_id = WindowId::new(42);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });

        // Invalid pattern should mark control with error style and skip diffing
        app_logic.handle_event(AppEvent::InputTextChanged {
            window_id,
            control_id: CONTROL_ID_TIMESTAMP_INPUT,
            text: "[".to_string(),
        });

        let command = app_logic
            .try_dequeue_command()
            .expect("expected style command for invalid regex");
        match command {
            PlatformCommand::ApplyStyleToControl {
                window_id: cmd_window,
                control_id,
                style_id,
            } => {
                assert_eq!(cmd_window, window_id);
                assert_eq!(control_id, CONTROL_ID_TIMESTAMP_INPUT);
                assert_eq!(style_id, StyleId::DefaultInputError);
            }
            other => panic!("unexpected command: {other:?}"),
        }
        assert!(
            app_logic.try_dequeue_command().is_none(),
            "no diff commands expected for invalid pattern"
        );

        let (_temp_dir, left_path, right_path) = create_test_files();

        // Load files - diff should be withheld because pattern invalid
        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_LEFT,
        });
        // Drain ShowOpenFileDialog command
        let _ = app_logic.try_dequeue_command();
        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(left_path.clone()),
        });
        assert!(app_logic.try_dequeue_command().is_none());

        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_RIGHT,
        });
        let _ = app_logic.try_dequeue_command();
        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(right_path.clone()),
        });
        assert!(
            app_logic.try_dequeue_command().is_none(),
            "still no diff commands while pattern invalid"
        );

        // Provide valid pattern -> style resets and diff executes
        app_logic.handle_event(AppEvent::InputTextChanged {
            window_id,
            control_id: CONTROL_ID_TIMESTAMP_INPUT,
            text: ".*".to_string(),
        });

        let restore_style = app_logic
            .try_dequeue_command()
            .expect("expected style reset command");
        match restore_style {
            PlatformCommand::ApplyStyleToControl {
                window_id: cmd_window,
                control_id,
                style_id,
            } => {
                assert_eq!(cmd_window, window_id);
                assert_eq!(control_id, CONTROL_ID_TIMESTAMP_INPUT);
                assert_eq!(style_id, StyleId::DefaultInput);
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let diff_calls = mock_diff_engine.calls();
        assert_eq!(
            diff_calls.len(),
            1,
            "diff engine should run once after valid regex"
        );

        // Diff commands follow
        let left_update = app_logic
            .try_dequeue_command()
            .expect("expected left viewer update after valid regex");
        let right_update = app_logic
            .try_dequeue_command()
            .expect("expected right viewer update after valid regex");

        assert!(matches!(
            left_update,
            PlatformCommand::SetViewerContent {
                control_id: CONTROL_ID_LEFT_VIEWER,
                ..
            }
        ));
        assert!(matches!(
            right_update,
            PlatformCommand::SetViewerContent {
                control_id: CONTROL_ID_RIGHT_VIEWER,
                ..
            }
        ));
    }

    fn create_test_files() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().expect("temp dir");
        let left_path = temp_dir.path().join("left.log");
        let right_path = temp_dir.path().join("right.log");
        {
            let mut left_file = File::create(&left_path).expect("left file");
            writeln!(left_file, "left-alpha").unwrap();
            writeln!(left_file, "left-beta").unwrap();
            left_file.flush().unwrap();
        }
        {
            let mut right_file = File::create(&right_path).expect("right file");
            writeln!(right_file, "right-alpha").unwrap();
            writeln!(right_file, "right-beta").unwrap();
            right_file.flush().unwrap();
        }

        (temp_dir, left_path, right_path)
    }
}
