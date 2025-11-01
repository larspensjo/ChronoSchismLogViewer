use crate::app_logic::ids::{
    CONTROL_ID_LEFT_VIEWER, CONTROL_ID_RIGHT_VIEWER, CONTROL_ID_TIMESTAMP_INPUT,
    LABEL_TIMESTAMP_PROMPT, MENU_ACTION_OPEN_LEFT, MENU_ACTION_OPEN_RIGHT, PANEL_INPUT_BAR,
    PANEL_VIEWER_CONTAINER,
};
use commanductui::types::{
    DockStyle, LabelClass, LayoutRule, MenuItemConfig, PlatformCommand, WindowId,
};
use commanductui::{Color, ControlStyle, StyleId};

/// Builds the static command list that describes the main application window.
/// This satisfies [CSV-UI-SideBySideV1] by defining the side-by-side viewer panels
/// and the timestamp input field at the top of the window.
pub fn build_main_window_layout(window_id: WindowId) -> Vec<PlatformCommand> {
    let file_menu_items = vec![
        MenuItemConfig {
            action: Some(MENU_ACTION_OPEN_LEFT),
            text: "Open &Left File...".to_string(),
            children: Vec::new(),
        },
        MenuItemConfig {
            action: Some(MENU_ACTION_OPEN_RIGHT),
            text: "Open &Right File...".to_string(),
            children: Vec::new(),
        },
    ];

    let menu_items = vec![MenuItemConfig {
        action: None,
        text: "&File".to_string(),
        children: file_menu_items,
    }];

    let mut commands = Vec::new();

    commands.push(PlatformCommand::DefineStyle {
        style_id: StyleId::DefaultInputError,
        style: ControlStyle {
            background_color: Some(Color { r: 0xB2, g: 0x1B, b: 0x1B }),
            text_color: None,
            font: None,
        },
    });

    commands.push(PlatformCommand::CreateMainMenu {
        window_id,
        menu_items,
    });

    commands.push(PlatformCommand::CreatePanel {
        window_id,
        parent_control_id: None,
        control_id: PANEL_INPUT_BAR,
    });
    commands.push(PlatformCommand::CreatePanel {
        window_id,
        parent_control_id: None,
        control_id: PANEL_VIEWER_CONTAINER,
    });

    commands.push(PlatformCommand::CreateLabel {
        window_id,
        parent_panel_id: PANEL_INPUT_BAR,
        control_id: LABEL_TIMESTAMP_PROMPT,
        initial_text: "Timestamp Pattern (regex):".to_string(),
        class: LabelClass::Default,
    });

    commands.push(PlatformCommand::CreateInput {
        window_id,
        parent_control_id: Some(PANEL_INPUT_BAR),
        control_id: CONTROL_ID_TIMESTAMP_INPUT,
        initial_text: String::new(),
        read_only: false,
        multiline: false,
        vertical_scroll: false,
    });

    commands.push(PlatformCommand::CreateInput {
        window_id,
        parent_control_id: Some(PANEL_VIEWER_CONTAINER),
        control_id: CONTROL_ID_LEFT_VIEWER,
        initial_text: String::new(),
        read_only: true,
        multiline: true,
        vertical_scroll: true,
    });

    commands.push(PlatformCommand::CreateInput {
        window_id,
        parent_control_id: Some(PANEL_VIEWER_CONTAINER),
        control_id: CONTROL_ID_RIGHT_VIEWER,
        initial_text: String::new(),
        read_only: true,
        multiline: true,
        vertical_scroll: true,
    });

    let layout_rules = vec![
        LayoutRule {
            control_id: PANEL_INPUT_BAR,
            parent_control_id: None,
            dock_style: DockStyle::Top,
            order: 0,
            fixed_size: Some(48),
            margin: (8, 8, 4, 8),
        },
        LayoutRule {
            control_id: PANEL_VIEWER_CONTAINER,
            parent_control_id: None,
            dock_style: DockStyle::Fill,
            order: 1,
            fixed_size: None,
            margin: (4, 8, 8, 8),
        },
        LayoutRule {
            control_id: LABEL_TIMESTAMP_PROMPT,
            parent_control_id: Some(PANEL_INPUT_BAR),
            dock_style: DockStyle::Left,
            order: 0,
            fixed_size: Some(220),
            margin: (8, 8, 8, 8),
        },
        LayoutRule {
            control_id: CONTROL_ID_TIMESTAMP_INPUT,
            parent_control_id: Some(PANEL_INPUT_BAR),
            dock_style: DockStyle::Fill,
            order: 1,
            fixed_size: None,
            margin: (8, 8, 8, 0),
        },
        LayoutRule {
            control_id: CONTROL_ID_LEFT_VIEWER,
            parent_control_id: Some(PANEL_VIEWER_CONTAINER),
            dock_style: DockStyle::ProportionalFill { weight: 1.0 },
            order: 0,
            fixed_size: None,
            margin: (8, 4, 8, 8),
        },
        LayoutRule {
            control_id: CONTROL_ID_RIGHT_VIEWER,
            parent_control_id: Some(PANEL_VIEWER_CONTAINER),
            dock_style: DockStyle::ProportionalFill { weight: 1.0 },
            order: 1,
            fixed_size: None,
            margin: (8, 8, 8, 4),
        },
    ];

    commands.push(PlatformCommand::DefineLayout {
        window_id,
        rules: layout_rules,
    });

    commands.push(PlatformCommand::ShowWindow { window_id });
    commands.push(PlatformCommand::SignalMainWindowUISetupComplete { window_id });

    commands
}
