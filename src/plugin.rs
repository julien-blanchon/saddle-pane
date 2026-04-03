use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy_ui_widgets::{checkbox_self_update, slider_self_update};

use crate::controls::{
    color::update_color_display,
    color_picker::{
        HsvPlaneMaterial, close_color_picker_on_click_outside, close_color_picker_on_escape,
        handle_color_picker_toggle, on_plane_drag, on_plane_drag_cancel, on_plane_drag_end,
        on_plane_drag_start, on_plane_press, position_color_picker, sync_color_picker_open,
        sync_hue_slider, update_picker_hex_text,
    },
    editing::{
        handle_pane_edit_focus, handle_swatch_click, on_pane_edit_keyboard, sync_pane_editing,
    },
    monitor::{update_monitor_graph, update_monitor_log_text, update_monitor_text},
    number::{on_number_step, update_number_step_repeat},
    scroll::{handle_ui_scroll, send_ui_scroll_events},
    on_pane_reset_button,
    select::{
        on_select_item_activate, on_select_menu_event, position_select_popup, sync_select_open,
        update_select_label,
    },
    slider::{sync_slider_to_control, update_slider_fill, update_slider_value_text},
    toggle::sync_toggle_to_control,
};
// Events (PaneChanged, PaneButtonPressed) are observer-based — triggered via commands.trigger()
use crate::icons::resolve_pane_icons;
use crate::registry::PaneControlRegistry;
use crate::store::PaneStore;
use crate::sync::{sync_controls_to_store, sync_store_to_controls};
use crate::search::{apply_search_filter, sync_search_input};
use crate::theme::{PaneTheme, apply_pane_theme_override, apply_theme};
use crate::ux::{
    TooltipState, cleanup_orphaned_tooltips, show_tooltip,
};

/// System sets for ordering pane systems.
#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum PaneSystems {
    /// Widget events → control components.
    Interaction,
    /// Control components → PaneStore + events.
    Sync,
    /// Control components → UI display updates.
    Display,
}

/// Main plugin for saddle_pane. Add this to your app to enable debug panes.
pub struct PanePlugin;

impl Plugin for PanePlugin {
    fn build(&self, app: &mut App) {
        // Embed CSS assets
        embedded_asset!(app, "style/tokens.css");
        embedded_asset!(app, "style/pane.css");
        embedded_asset!(app, "style/folder.css");
        embedded_asset!(app, "style/slider.css");
        embedded_asset!(app, "style/toggle.css");
        embedded_asset!(app, "style/button.css");
        embedded_asset!(app, "style/number.css");
        embedded_asset!(app, "style/text.css");
        embedded_asset!(app, "style/select.css");
        embedded_asset!(app, "style/select_popup.css");
        embedded_asset!(app, "style/color.css");
        embedded_asset!(app, "style/separator.css");
        embedded_asset!(app, "style/themes/dark.css");
        embedded_asset!(app, "style/monitor.css");
        embedded_asset!(app, "style/tab.css");
        // interval.css is now in the external saddle_pane_interval crate
        // Color picker assets
        embedded_asset!(app, "style/color_picker.css");
        embedded_asset!(app, "style/color_picker.wgsl");

        // UI material for color plane shader
        app.add_plugins(UiMaterialPlugin::<HsvPlaneMaterial>::default());

        // Resources
        app.init_resource::<PaneStore>();
        app.init_resource::<PaneTheme>();
        app.init_resource::<PaneControlRegistry>();
        app.init_resource::<TooltipState>();

        // Events are observer-based — no registration needed.
        // Users observe via: app.add_observer(|ev: On<PaneChanged>| { ... })

        // Global widget self-update observers
        app.add_observer(checkbox_self_update);
        app.add_observer(slider_self_update);

        // Select/dropdown observers (MenuEvent for trigger, Activate for items)
        app.add_observer(on_select_menu_event);
        app.add_observer(on_select_item_activate);

        // Number step buttons observer
        app.add_observer(on_number_step);

        // Pane editing observer (keyboard input)
        app.add_observer(on_pane_edit_keyboard);

        // Reset button observer
        app.add_observer(on_pane_reset_button);

        // Scroll: mouse wheel → UiScroll → ScrollPosition
        app.add_observer(handle_ui_scroll);
        app.add_systems(Update, send_ui_scroll_events);

        // Color picker plane interaction observers
        app.add_observer(on_plane_press);
        app.add_observer(on_plane_drag_start);
        app.add_observer(on_plane_drag);
        app.add_observer(on_plane_drag_end);
        app.add_observer(on_plane_drag_cancel);

        // System set ordering
        app.configure_sets(
            PostUpdate,
            (
                PaneSystems::Interaction,
                PaneSystems::Sync,
                PaneSystems::Display,
            )
                .chain(),
        );

        // PreUpdate: slider visual sync (fill + text)
        app.add_systems(PreUpdate, (update_slider_fill, update_slider_value_text));

        // Update: click-to-focus, swatch click, select popup sync, step repeat, color picker, icons, theme
        app.add_systems(
            Update,
            (
                handle_pane_edit_focus,
                handle_swatch_click,
                sync_select_open,
                update_number_step_repeat,
                handle_color_picker_toggle,
                sync_color_picker_open,
                close_color_picker_on_escape,
                close_color_picker_on_click_outside,
                resolve_pane_icons,
                apply_theme,
                apply_pane_theme_override,
                sync_search_input,
                apply_search_filter,
                show_tooltip,
                cleanup_orphaned_tooltips,
            ),
        );

        // PostUpdate: interaction sync (widget → control components) + editing
        app.add_systems(
            PostUpdate,
            (
                sync_slider_to_control,
                sync_toggle_to_control,
                sync_pane_editing,
                sync_hue_slider,
                position_color_picker,
                position_select_popup,
            )
                .in_set(PaneSystems::Interaction),
        );

        // PostUpdate: store sync (control components ↔ store + events)
        app.add_systems(
            PostUpdate,
            (sync_controls_to_store, sync_store_to_controls)
                .chain()
                .in_set(PaneSystems::Sync),
        );

        // PostUpdate: display sync (control components → UI)
        app.add_systems(
            PostUpdate,
            (
                update_select_label,
                update_color_display,
                update_picker_hex_text,
                update_monitor_text,
                update_monitor_log_text,
                update_monitor_graph,
            )
                .in_set(PaneSystems::Display),
        );
    }
}
