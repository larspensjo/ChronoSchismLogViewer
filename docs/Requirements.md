### Requirement Specification for ChronoSchism Log Viewer

Here is a full set of initial requirements, following the format you requested.

#### Core Application Functionality
*   `[CSV-Core-CompareV1]` The application must be able to load and compare two distinct log files.
*   `[CSV-Core-IgnoreTSV1]` The comparison logic must be able to ignore differences that match a user-defined timestamp pattern.
*   `[CSV-Core-TSPatternV1]` The application shall provide a mechanism for the user to define the timestamp pattern, preferably using regular expressions.
*   `[CSV-Core-RegexCacheV1]` The timestamp parsing logic shall cache compiled regular expressions so repeated use of the same pattern avoids recompilation overhead.
*   `[CSV-Core-LargeFileV1]` The application should handle large log files gracefully, without freezing the UI during file loading or comparison.

#### Diff Algorithm
*   `[CSV-Diff-HeckelV1]` The core diffing logic must be implemented using Paul Heckel's Diff Algorithm to correctly identify added, deleted, unchanged, and moved blocks of text.

#### File Handling
*   `[CSV-File-LoadV1]` The application must provide separate actions to load the "left" file and the "right" file for comparison.

#### User Interface (UI)
*   `[CSV-UI-SideBySideV1]` The comparison shall be displayed in a side-by-side view, with the left file in a left-hand panel and the right file in a right-hand panel.
*   `[CSV-UI-HighlightV1]` Differences between the files must be visually indicated using color highlighting: one color for additions, one for deletions, and one for unchanged lines.
*   `[CSV-UI-MovedBlocksV1]` Text blocks that have been moved must be visually indicated, for instance, by connecting their old and new locations with lines or bands.
*   `[CSV-UI-TimestampInputV1]` There shall be a dedicated input field for the user to enter and apply a timestamp regex pattern.

#### User Experience (UX)
*   `[CSV-UX-LinkedScrollV1]` The vertical scroll bars of the two comparison panels must be linked, so that scrolling one panel scrolls the other in sync.
*   `[CSV-UX-ResponsiveV1]` The application UI must remain responsive during file operations and diff calculations, making use of background processing where appropriate.

#### Technical Requirements
*   `[CSV-Tech-RustV1]` The application shall be implemented in the Rust programming language.
*   `[CSV-Tech-CommanDuctV1]` The user interface shall be implemented using the `CommanDuctUI` library, following its command-event pattern.
*   `[CSV-Tech-DIV1]` The application's architecture must use Dependency Injection, with core logic abstracted behind traits, mirroring the `SourcePacker` reference.
*   `[CSV-Tech-UnitTestsV1]` All core and application logic must be accompanied by a thorough suite of unit tests, using mock objects to isolate components.

#### Software design requirements
*   `[CSV-Tech-EncapsulationV1]` Structs shall keep their fields private to preserve encapsulation, except when the struct is intentionally used as a passive data container.
*   `[CSV-Tech-TraceabilityV1]` Requirement identifiers shall appear in implementation comments and corresponding unit tests to aid traceability between code, tests, and documented requirements.
