use std::error::Error;
use std::sync::{Arc, Mutex};

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
    let _ = env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .try_init();

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
