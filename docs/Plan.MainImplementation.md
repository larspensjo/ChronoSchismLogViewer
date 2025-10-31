### Step-by-Step Implementation Plan

#### Phase 1: Project Setup & Structure

1.  **Create the Project:**
    *   Run `cargo new chrono_schism_log_viewer` to create the new Rust project.
    *   Initialize a git repository: `git init`.

2.  **Integrate `CommanDuctUI`:**
    *   Add your `CommanDuctUI` project as a git submodule: `git submodule add <url_to_your_commanductui_repo> src/CommanDuctUI`.
    *   Update the main `Cargo.toml` to include it as a local dependency:
        ```toml
        [dependencies]
        commanductui = { path = "src/CommanDuctUI" }
        # ... other dependencies will go here, like `regex`
        ```

3.  **Establish Module Structure:**
    *   Create the main source folders and files, mimicking `SourcePacker`:
        ```
        src/
        ├── core/                // Platform-agnostic business logic
        │   ├── mod.rs
        │   ├── diff_engine.rs
        │   └── timestamp_parser.rs
        ├── app_logic/           // Presenter/UI logic
        │   ├── mod.rs
        │   ├── handler.rs
        │   └── handler_tests.rs
        ├── ui_description_layer.rs // Static UI layout definition
        └── main.rs
        ```

---

#### Phase 2: Core Logic (The "Model")

This phase has **no dependency on `CommanDuctUI`** and should be fully unit-tested.

1.  **Define Core Traits (`[CSV-Tech-DIV1]`):**
    *   In `core/timestamp_parser.rs`, define `trait TimestampParserOperations` with a method `strip_timestamps(&self, lines: &[String], pattern: &str) -> Result<Vec<String>, ...>`.
    *   In `core/diff_engine.rs`, define `trait DiffEngineOperations` with a method `compute_diff(&self, lines_a: &[String], lines_b: &[String]) -> DiffResult`. First, define helper structs like `DiffLine` (with an enum `DiffState::{Added, Deleted, Unchanged, Moved}`), and `DiffResult`.

2.  **Implement the Heckel Algorithm (`[CSV-Diff-HeckelV1]`):**
    *   In `core/diff_engine.rs`, create `struct HeckelDiffEngine`.
    *   Implement `DiffEngineOperations` for it. This is where you will implement the logic of Heckel's algorithm. This is the most complex part of the core logic.
    *   Add thorough unit tests in the same file to validate the algorithm with various inputs (additions, deletions, moves, and mixed changes).

3.  **Implement the Timestamp Parser (`[CSV-Core-TSPatternV1]`):**
    *   In `core/timestamp_parser.rs`, create `struct CoreTimestampParser`.
    *   Implement `TimestampParserOperations` for it. The `strip_timestamps` method will use the `regex` crate to remove pattern matches from each line.
    *   Add unit tests to verify that various timestamp formats are correctly removed.

---

#### Phase 3: Application Logic (The "Presenter")

This layer connects the `core` logic to the UI layer's commands and events.

1.  **Define `AppLogic` State and Dependencies:**
    *   In `app_logic/handler.rs`, create `struct AppLogic`.
    *   Its `new` method will take `Arc<dyn DiffEngineOperations>` and `Arc<dyn TimestampParserOperations>` as arguments.
    *   It will hold state such as `Option<PathBuf>` for the left and right files, the current `String` for the timestamp pattern, and the `Vec<DiffLine>` results.

2.  **Implement `PlatformEventHandler` (`[CSV-Tech-CommanDuctV1]`):**
    *   Implement the `PlatformEventHandler` trait for `AppLogic`.
    *   **`handle_event`:**
        *   Match `AppEvent::MenuActionClicked`. For "Open Left" and "Open Right", issue a `PlatformCommand::ShowOpenFileDialog`.
        *   Match `AppEvent::FileOpenDialogCompleted`. Store the resulting path in the appropriate state field (`left_file_path` or `right_file_path`).
        *   After a file is loaded, if both paths are now `Some`, trigger the diffing process.

3.  **Implement the Diffing Workflow:**
    *   Create a private method in `AppLogic`, like `_trigger_diff`.
    *   This method will:
        1.  Read the lines from both files.
        2.  Call the `timestamp_parser` to strip timestamps.
        3.  Call the `diff_engine` to get the `DiffResult`.
        4.  Store the result in `AppLogic`'s state.
        5.  Enqueue `PlatformCommand`s to update the two viewer panels with the diff results. For now, this can just be setting the text content of two `Input` controls.

4.  **Write Unit Tests (`[CSV-Tech-UnitTestsV1]`):**
    *   In `app_logic/handler_tests.rs`, create mock implementations: `MockDiffEngine` and `MockTimestampParser`.
    *   Write a test for the main workflow:
        *   **Arrange:** Create `AppLogic` with mocks.
        *   **Act:** Simulate `AppEvent`s for opening two files.
        *   **Assert:** Verify that the mocks' methods were called. Check that `try_dequeue_command` returns the expected `PlatformCommand`s to update the UI with the mock diff results.

---

#### Phase 4: UI Description and Entry Point

This phase wires everything together into a runnable application.

1.  **Describe the UI Layout (`[CSV-UI-SideBySideV1]`):**
    *   In `ui_description_layer.rs`, create a `build_main_window_layout` function.
    *   It will return a `Vec<PlatformCommand>` that defines the full static UI:
        *   A `CreateMainMenu` command with "File" > "Open Left File..." and "Open Right File..." menu items, wired to new `MenuAction`s.
        *   `CreatePanel` commands to set up the main layout containers (e.g., a top panel for inputs, a main panel for viewers).
        *   A single-line `CreateInput` control for the timestamp regex pattern (`[CSV-UI-TimestampInputV1]`).
        *   Two large, read-only, multi-line `CreateInput` controls to serve as the "left" and "right" log file viewers.
        *   A `DefineLayout` command with `LayoutRule`s to position everything (e.g., regex input at the top, viewers side-by-side filling the remaining space).

2.  **Create the Entry Point:**
    *   In `main.rs`, write the `main` function. It will be very similar to `SourcePacker`'s:
        1.  Initialize logging.
        2.  Instantiate `CoreDiffEngine` and `CoreTimestampParser`.
        3.  Instantiate `AppLogic` with the `Arc`s of the core services.
        4.  Instantiate `PlatformInterface`.
        5.  Call `platform_interface.create_window()`.
        6.  Get the layout commands from `ui_description_layer::build_main_window_layout()`.
        7.  Call `platform_interface.main_event_loop()` with the `AppLogic` instance and the initial commands.

At this point, you will have a functional-but-basic application. You can load two files, and the text content of the two viewers will update to show the raw diff results.

---

#### Phase 5: Refinement and Advanced UX Features

Now, build upon the working foundation to implement the more advanced visual features.

1.  **Custom Diff View Control (`[CSV-UI-HighlightV1]`):**
    *   Instead of using two simple `Input` controls, you will need a custom-drawn control. The best approach is to enhance `CommanDuctUI` with a new `DiffView` control type.
    *   This control would internally handle the `WM_PAINT` message. Your `AppLogic` would send it a custom command like `PlatformCommand::SetDiffResults { control_id, results }`.
    *   The control's handler would then iterate through the `DiffResult` and draw each line with the appropriate background color.

2.  **Linked Scrolling (`[CSV-UX-LinkedScrollV1]`):**
    *   In the `WndProc` for your custom `DiffView` control (or the main window), handle `WM_VSCROLL`.
    *   When a scroll event is received from one panel, send a corresponding scroll message (`WM_VSCROLL` or `SetScrollPos`) to the other panel to keep them synchronized.

3.  **Moved Block Visualization (`[CSV-UI-MovedBlocksV1]`):**
    *   This is the most visually complex feature. During your `WM_PAINT` handling in the custom `DiffView` control, after drawing the text lines, iterate through the moved blocks identified in your `DiffResult`.
    *   For each moved block, get the screen coordinates of its original position in one panel and its new position in the other.
    *   Use GDI functions (e.g., `MoveToEx`, `LineTo`, or `Polygon`) to draw connecting lines or semi-transparent polygons between these two areas.

4.  **Implement Debouncing for Timestamp Input:**
    *   Enhance the `AppLogic` to handle `AppEvent::InputTextChanged` from the timestamp regex control.
    *   Instead of validating immediately, start a short platform timer. If another text change event arrives, reset the timer.
    *   When the timer fires, validate the regex pattern. This prevents showing error popups while the user is actively typing, dramatically improving the user experience.
