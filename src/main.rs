use std::error::Error;
use std::sync::{Arc, Mutex};

use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use time::macros::format_description;

use ChronoSchismLogViewer::app_logic::handler::AppLogic;
use ChronoSchismLogViewer::core::diff_engine::{DiffEngineOperations, HeckelDiffEngine};
use ChronoSchismLogViewer::core::timestamp_parser::{
    CoreTimestampParser, TimestampParserOperations,
};
use ChronoSchismLogViewer::ui_description_layer;
use commanductui::PlatformInterface;
use commanductui::types::{PlatformEventHandler, UiStateProvider, WindowConfig};

const APP_NAME: &str = "ChronoSchism Log Viewer";
const APP_CLASS_NAME: &str = "ChronoSchismLogViewer";

fn main() {
    if let Err(err) = run() {
        log::error!("Application error: {err}");
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    initialize_logging(LevelFilter::Debug);

    log::info!("Starting {APP_NAME}");

    let diff_engine: Arc<dyn DiffEngineOperations> = Arc::new(HeckelDiffEngine::new());
    let timestamp_parser: Arc<dyn TimestampParserOperations> = Arc::new(CoreTimestampParser::new());

    let shared_logic = Arc::new(Mutex::new(AppLogic::new(diff_engine, timestamp_parser)));

    let event_handler: Arc<Mutex<dyn PlatformEventHandler>> = shared_logic.clone();
    let ui_state_provider: Arc<Mutex<dyn UiStateProvider>> = shared_logic;

    let platform = PlatformInterface::new(APP_CLASS_NAME.to_string())?;

    let window_id = platform.create_window(WindowConfig {
        title: APP_NAME,
        width: 1280,
        height: 900,
    })?;

    let layout_commands = ui_description_layer::build_main_window_layout(window_id);

    platform.main_event_loop(event_handler, ui_state_provider, layout_commands)?;

    Ok(())
}

// [CSV-Tech-LogFileV1]
pub fn initialize_logging(log_level: LevelFilter) {
    let log_file_path = "ChronoSchismLogViewer.log";
    match std::fs::File::create(log_file_path) {
        Ok(file) => {
            let mut config_builder = ConfigBuilder::new();

            if let Err(err) = config_builder.set_time_offset_to_local() {
                eprintln!("Warning: Failed to set local time offset: {err:?}");
            }

            let config = config_builder
                .set_thread_level(LevelFilter::Off)
                .set_location_level(LevelFilter::Debug)
                .set_time_format_custom(format_description!(
                    "[hour]:[minute]:[second].[subsecond digits:3]"
                ))
                .build();

            if let Err(err) =
                simplelog::CombinedLogger::init(vec![WriteLogger::new(log_level, config, file)])
            {
                eprintln!("Failed to initialize file logger: {err}");
            }
        }
        Err(err) => {
            eprintln!("Failed to create log file '{log_file_path}': {err}");
        }
    }

    println!("Logging initialized to file: {log_file_path}, at level {log_level}");
}
