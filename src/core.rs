pub mod diff_engine;
pub mod path_utils;
pub mod settings;
pub mod settings_manager;
pub mod timestamp_parser;

pub use diff_engine::{
    ComparableLine, DiffEngineOperations, DiffLine, DiffResult, DiffState, DiffStatistics,
    LineContent, MovedBlock,
};
pub use settings::AppSettings;
pub use settings_manager::{CoreSettingsManager, SettingsManagerOperations};
pub use timestamp_parser::{TimestampParserError, TimestampParserOperations};
