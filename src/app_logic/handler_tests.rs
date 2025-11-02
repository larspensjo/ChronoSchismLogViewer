#[cfg(test)]
mod tests {
    use crate::app_logic::handler::AppLogic;
    use crate::app_logic::ids::{
        CONTROL_ID_LEFT_VIEWER, CONTROL_ID_RIGHT_VIEWER, CONTROL_ID_TIMESTAMP_INPUT,
        MENU_ACTION_EXIT, MENU_ACTION_OPEN_LEFT, MENU_ACTION_OPEN_RIGHT,
    };
    use crate::core::{
        AppSettings, ComparableLine, DiffEngineOperations, DiffLine, DiffState, LineContent,
        SettingsManagerOperations, TimestampParserOperations,
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

    #[derive(Default)]
    struct MockSettingsManager {
        saved: Mutex<Vec<(String, AppSettings)>>,
        load_response: Mutex<AppSettings>,
    }

    impl MockSettingsManager {
        fn saved_snapshots(&self) -> Vec<(String, AppSettings)> {
            self.saved.lock().unwrap().clone()
        }
    }

    impl SettingsManagerOperations for MockSettingsManager {
        fn save_settings(
            &self,
            app_name: &str,
            settings: &AppSettings,
        ) -> Result<(), std::io::Error> {
            self.saved
                .lock()
                .unwrap()
                .push((app_name.to_string(), settings.clone()));
            Ok(())
        }

        fn load_settings(&self, _app_name: &str) -> Result<AppSettings, std::io::Error> {
            Ok(self.load_response.lock().unwrap().clone())
        }
    }

    fn snapshot(lines: &[ComparableLine]) -> Vec<(&str, &str)> {
        lines
            .iter()
            .map(|line| (line.original_text.as_str(), line.comparable_text.as_str()))
            .collect()
    }

    fn drain_commands(app_logic: &mut AppLogic) {
        while app_logic.try_dequeue_command().is_some() {}
    }

    fn load_files_and_pattern(
        app_logic: &mut AppLogic,
        window_id: WindowId,
        left_path: &PathBuf,
        right_path: &PathBuf,
        pattern: &str,
    ) {
        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_LEFT,
        });
        let _ = app_logic.try_dequeue_command();
        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(left_path.clone()),
        });
        drain_commands(app_logic);

        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_RIGHT,
        });
        let _ = app_logic.try_dequeue_command();
        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(right_path.clone()),
        });
        drain_commands(app_logic);

        app_logic.handle_event(AppEvent::InputTextChanged {
            window_id,
            control_id: CONTROL_ID_TIMESTAMP_INPUT,
            text: pattern.to_string(),
        });
        drain_commands(app_logic);
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
        let settings_manager: Arc<dyn SettingsManagerOperations> =
            Arc::new(MockSettingsManager::default());
        let mut app_logic =
            AppLogic::new(diff_engine, timestamp_parser, settings_manager, "test-app");

        let window_id = WindowId::new(7);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });
        // [CSV-Tech-SettingsPersistenceV1] Drain the initial input sync command produced by loading settings.
        app_logic.try_dequeue_command();

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
        let settings_manager: Arc<dyn SettingsManagerOperations> =
            Arc::new(MockSettingsManager::default());
        let mut app_logic =
            AppLogic::new(diff_engine, timestamp_parser, settings_manager, "test-app");

        let window_id = WindowId::new(42);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });
        // [CSV-Tech-SettingsPersistenceV1] Initial settings load syncs the input field.
        app_logic.try_dequeue_command();

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

    #[test]
    fn file_exit_menu_closes_window_and_persists_settings() {
        let diff_lines = vec![DiffLine::new(
            DiffState::Unchanged,
            Some(LineContent::new(1, "alpha")),
            Some(LineContent::new(1, "alpha")),
        )];
        let mock_diff_engine = Arc::new(MockDiffEngine::new(diff_lines));
        let mock_timestamp_parser = Arc::new(MockTimestampParser::default());
        let settings_manager = Arc::new(MockSettingsManager::default());

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let settings_arc: Arc<dyn SettingsManagerOperations> = settings_manager.clone();
        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser, settings_arc, "test-app");

        let window_id = WindowId::new(77);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });
        drain_commands(&mut app_logic);

        let (_temp_dir, left_path, right_path) = create_test_files();
        load_files_and_pattern(&mut app_logic, window_id, &left_path, &right_path, ".*");

        // [CSV-UI-ExitCommandV1] File/Exit should initiate shutdown.
        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_EXIT,
        });

        let close_command = app_logic
            .try_dequeue_command()
            .expect("expected CloseWindow command");
        match close_command {
            PlatformCommand::CloseWindow {
                window_id: cmd_window,
            } => assert_eq!(cmd_window, window_id),
            other => panic!("unexpected command: {other:?}"),
        }

        let saved = settings_manager.saved_snapshots();
        assert_eq!(saved.len(), 1, "exit should persist settings immediately");
        let (app_name, snapshot) = &saved[0];
        assert_eq!(app_name, "test-app");
        assert_eq!(snapshot.left_file_path(), Some(&left_path));
        assert_eq!(snapshot.right_file_path(), Some(&right_path));
        assert_eq!(snapshot.timestamp_pattern(), ".*");
        let history: Vec<&str> = snapshot
            .timestamp_history()
            .iter()
            .map(|entry| entry.as_str())
            .collect();
        assert_eq!(
            history,
            vec![".*"],
            "[CSV-UX-TimestampHistoryV1] exit preserves latest pattern history"
        );
    }

    #[test]
    fn linked_scrolling_propagates_to_other_viewer() {
        // Arrange
        let mock_diff_engine = Arc::new(MockDiffEngine::new(vec![]));
        let mock_timestamp_parser = Arc::new(MockTimestampParser::default());
        let settings_manager = Arc::new(MockSettingsManager::default());

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let settings_arc: Arc<dyn SettingsManagerOperations> = settings_manager.clone();

        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser, settings_arc, "test-app");
        let window_id = WindowId::new(1);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });
        drain_commands(&mut app_logic);

        // Act
        app_logic.handle_event(AppEvent::ControlScrolled {
            window_id,
            control_id: CONTROL_ID_LEFT_VIEWER,
            vertical_pos: 50,
            horizontal_pos: 0,
        });

        // Assert
        let command = app_logic
            .try_dequeue_command()
            .expect("expected SetScrollPosition command");
        match command {
            PlatformCommand::SetScrollPosition {
                control_id,
                vertical_pos,
                horizontal_pos,
                ..
            } => {
                assert_eq!(control_id, CONTROL_ID_RIGHT_VIEWER);
                assert_eq!(vertical_pos, 50);
                assert_eq!(horizontal_pos, 0);
            }
            other => panic!("Unexpected command generated: {other:?}"),
        }

        assert!(
            app_logic.try_dequeue_command().is_none(),
            "only one command should be generated"
        );
    }

    #[test]
    fn window_close_event_requests_shutdown() {
        let diff_lines = vec![DiffLine::new(
            DiffState::Unchanged,
            Some(LineContent::new(1, "alpha")),
            Some(LineContent::new(1, "alpha")),
        )];
        let mock_diff_engine = Arc::new(MockDiffEngine::new(diff_lines));
        let mock_timestamp_parser = Arc::new(MockTimestampParser::default());
        let settings_manager = Arc::new(MockSettingsManager::default());

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let settings_arc: Arc<dyn SettingsManagerOperations> = settings_manager.clone();
        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser, settings_arc, "test-app");

        let window_id = WindowId::new(88);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });
        drain_commands(&mut app_logic);

        let (_temp_dir, left_path, right_path) = create_test_files();
        load_files_and_pattern(&mut app_logic, window_id, &left_path, &right_path, "\\d+");

        // [CSV-UI-ExitCommandV1] Closing via the window chrome should mirror File/Exit.
        app_logic.handle_event(AppEvent::WindowCloseRequestedByUser { window_id });

        let close_command = app_logic
            .try_dequeue_command()
            .expect("expected CloseWindow command on close request");
        match close_command {
            PlatformCommand::CloseWindow {
                window_id: cmd_window,
            } => assert_eq!(cmd_window, window_id),
            other => panic!("unexpected command: {other:?}"),
        }

        let saved = settings_manager.saved_snapshots();
        assert_eq!(saved.len(), 1, "close request should persist settings");
        let snapshot = &saved[0].1;
        assert_eq!(snapshot.left_file_path(), Some(&left_path));
        assert_eq!(snapshot.right_file_path(), Some(&right_path));
        assert_eq!(snapshot.timestamp_pattern(), "\\d+");
    }

    #[test]
    fn settings_persisted_on_quit_captures_recent_history() {
        let diff_lines = vec![DiffLine::new(
            DiffState::Unchanged,
            Some(LineContent::new(1, "alpha")),
            Some(LineContent::new(1, "alpha")),
        )];
        let mock_diff_engine = Arc::new(MockDiffEngine::new(diff_lines));
        let mock_timestamp_parser = Arc::new(MockTimestampParser::default());
        let settings_manager = Arc::new(MockSettingsManager::default());

        let diff_engine: Arc<dyn DiffEngineOperations> = mock_diff_engine.clone();
        let timestamp_parser: Arc<dyn TimestampParserOperations> = mock_timestamp_parser.clone();
        let settings_arc: Arc<dyn SettingsManagerOperations> = settings_manager.clone();
        let mut app_logic = AppLogic::new(diff_engine, timestamp_parser, settings_arc, "test-app");

        let window_id = WindowId::new(101);
        app_logic.handle_event(AppEvent::MainWindowUISetupComplete { window_id });
        // [CSV-Tech-SettingsPersistenceV1] Settings load synchronizes UI state on startup.
        drain_commands(&mut app_logic);

        let (_temp_dir, left_path, right_path) = create_test_files();

        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_LEFT,
        });
        let _ = app_logic.try_dequeue_command();
        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(left_path.clone()),
        });
        drain_commands(&mut app_logic);

        app_logic.handle_event(AppEvent::MenuActionClicked {
            action_id: MENU_ACTION_OPEN_RIGHT,
        });
        let _ = app_logic.try_dequeue_command();
        app_logic.handle_event(AppEvent::FileOpenProfileDialogCompleted {
            window_id,
            result: Some(right_path.clone()),
        });
        drain_commands(&mut app_logic);

        for pattern in ["one", "two", "three", "four", "five", "two", "six"] {
            app_logic.handle_event(AppEvent::InputTextChanged {
                window_id,
                control_id: CONTROL_ID_TIMESTAMP_INPUT,
                text: pattern.to_string(),
            });
            // [CSV-UX-TimestampHistoryV1] Valid pattern inputs update the MRU collection.
            drain_commands(&mut app_logic);
        }

        PlatformEventHandler::on_quit(&mut app_logic);

        let saved = settings_manager.saved_snapshots();
        assert_eq!(saved.len(), 1, "expected a single persistence attempt");
        let (app_name, snapshot) = &saved[0];
        assert_eq!(app_name, "test-app");
        assert_eq!(snapshot.left_file_path(), Some(&left_path));
        assert_eq!(snapshot.right_file_path(), Some(&right_path));
        assert_eq!(snapshot.timestamp_pattern(), "six");

        let history: Vec<&str> = snapshot
            .timestamp_history()
            .iter()
            .map(|entry| entry.as_str())
            .collect();
        assert_eq!(
            history,
            vec!["six", "two", "five", "four", "three"],
            "history stores a five-entry MRU per [CSV-UX-TimestampHistoryV1]"
        );
    }
}
