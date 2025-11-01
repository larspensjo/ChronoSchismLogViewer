pub mod diff_engine;
pub mod timestamp_parser;

pub use diff_engine::{
    ComparableLine, DiffEngineOperations, DiffLine, DiffResult, DiffState, DiffStatistics,
    LineContent, MovedBlock,
};
pub use timestamp_parser::{TimestampParserError, TimestampParserOperations};
