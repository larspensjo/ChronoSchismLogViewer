#[cfg(test)]
mod tests {
    use crate::app_logic::handler::AppLogic;
    use crate::app_logic::ids::{CONTROL_ID_LEFT_VIEWER, CONTROL_ID_RIGHT_VIEWER};
    use crate::core::{
        DiffEngineOperations, DiffLine, DiffState, LineContent, TimestampParserOperations,
    };
    use commanductui::types::{AppEvent, MenuAction, PlatformCommand, WindowId};
    use commanductui::PlatformEventHandler;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[derive(Default)]
    struct MockTimestampParser {
        calls: Mutex<Vec<(Vec<String>, String)>>,
    }

    impl MockTimestampParser {
        fn calls(&self) -> Vec<(Vec<String>, String)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl TimestampParserOperations for MockTimestampParser {
        fn strip_timestamps(
            &self,
            lines: &[String],
            pattern: &str,
        ) -> Result<Vec<String>, crate::core::TimestampParserError> {
            let mut guard = self.calls.lock().unwrap();
            guard.push((lines.to_vec(), pattern.to_string()));
            Ok(lines.to_vec())
        }
    }

    struct MockDiffEngine {
        calls: Mutex<Vec<(Vec<String>, Vec<String>)>>,
        lines_to_return: Vec<DiffLine>,
    }

    impl MockDiffEngine {
        fn new(lines_to_return: Vec<DiffLine>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                lines_to_return,
            }
        }

        fn calls(&self) -> Vec<(Vec<String>, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl DiffEngineOperations for MockDiffEngine {
        fn compute_diff(&self, lines_a: &[String], lines_b: &[String]) -> crate::core::DiffResult {
            let mut guard = self.calls.lock().unwrap();
            guard.push((lines_a.to_vec(), lines_b.to_vec()));
            crate::core::DiffResult::new(self.lines_to_return.clone())
        }
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
        let mock_timestamp_parser = Arc::new(MockTimestampParser::default());

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser);

        let window_id = WindowId::new(7);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });

        let (_temp_dir, left_path, right_path) = create_test_files();

        // Act: open left file
        app_logic.handle_event(AppEvent::MenuActionClicked {
            action: MenuAction::OpenLeftLogFile,
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
            action: MenuAction::OpenRightLogFile,
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
            diff_calls[0],
            (
                vec!["left-alpha".into(), "left-beta".into()],
                vec!["right-alpha".into(), "right-beta".into()]
            )
        );
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
