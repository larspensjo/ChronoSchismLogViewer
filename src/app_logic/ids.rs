use commanductui::types::{ControlId, MenuActionId};

pub const CONTROL_ID_TIMESTAMP_INPUT: ControlId = ControlId::new(1_001);
pub const CONTROL_ID_LEFT_VIEWER: ControlId = ControlId::new(1_010);
pub const CONTROL_ID_RIGHT_VIEWER: ControlId = ControlId::new(1_011);

pub const PANEL_INPUT_BAR: ControlId = ControlId::new(2_001);
pub const PANEL_VIEWER_CONTAINER: ControlId = ControlId::new(2_010);

pub const LABEL_TIMESTAMP_PROMPT: ControlId = ControlId::new(3_001);

pub const MENU_ACTION_OPEN_LEFT: MenuActionId = MenuActionId(1);
pub const MENU_ACTION_OPEN_RIGHT: MenuActionId = MenuActionId(2);
