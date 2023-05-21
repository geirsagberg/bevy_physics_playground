use bevy_egui::{egui, EguiContexts};
use bevy::prelude::{EventWriter, Res};
use bevy_egui::egui::Window;
use strum::IntoEnumIterator;
use crate::{Mode, Tool, ToolEvent};

pub(crate) fn update_ui(
    mut egui_contexts: EguiContexts,
    mode: Res<Mode>,
    mut event_sender: EventWriter<ToolEvent>,
) {
    let ctx = egui_contexts.ctx_mut();

    Window::new("Physics").show(ctx, |ui| {
        ui.label(format!("Mode: {:?}", *mode));

        let mut add_button = |label: &str, tool: Tool| {
            ui.add_enabled_ui(*mode == Mode::Default, |ui| {
                if ui.button(label).clicked() {
                    event_sender.send(ToolEvent {
                        tool
                    });
                }
            });
        };


        for tool in Tool::iter() {
            add_button(tool.label(), tool);
        }
    });
}
