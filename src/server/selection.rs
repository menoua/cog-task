use crate::server::{Page, Progress, Server, ServerSignal};
use crate::style::text::{body, button1, tooltip};
use crate::style::{
    style_ui, Style, CUSTOM_BLUE, CUSTOM_ORANGE, CUSTOM_RED, FOREST_GREEN, TEXT_SIZE_DIALOGUE_BODY,
    TEXT_SIZE_DIALOGUE_TITLE,
};
use crate::template::{center_x, header_body_controls};
use eframe::egui;
use eframe::egui::{Direction, Label, Layout, Pos2, RichText, ScrollArea, Vec2, Window};
use egui_extras::{Size, StripBuilder};
use std::thread;

impl Server {
    pub(crate) fn show_selection(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(matches!(self.status, Progress::None), |ui| {
            header_body_controls(ui, |strip| {
                strip.cell(|ui| {
                    ui.centered_and_justified(|ui| ui.heading(self.task.title()));
                });
                strip.empty();
                strip.strip(|builder| {
                    center_x(builder, 1520.0, |ui| self.show_selection_blocks(ui));
                });
                strip.empty();
                strip.strip(|builder| self.show_selection_controls(builder));
            });
        });

        if !matches!(self.status, Progress::None) {
            self.show_selection_status(ui.ctx());
        }

        if ui.input().key_pressed(egui::Key::Escape) && !matches!(self.status, Progress::None) {
            self.blocks.get_mut(self.active_block.unwrap()).unwrap().1 =
                std::mem::replace(&mut self.status, Progress::None);
        }
    }

    fn show_selection_blocks(&mut self, ui: &mut egui::Ui) {
        enum Interaction {
            None,
            StartBlock(usize),
        }

        let mut interaction = Interaction::None;

        let names = self.task.block_labels();
        let is_done: Vec<_> = self.blocks.iter().map(|(_, done)| done).collect();

        let cols = self.config().blocks_per_row() as usize;
        let rows = (names.len() + cols - 1) / cols;
        let row_height = 70.0;
        let height = (row_height + 10.0) * rows as f32 + 20.0;
        let (col_width, col_spacing) = match cols {
            1 => (720.0, 0.0),
            2 => (600.0, 120.0),
            3 => (440.0, 60.0),
            _ => (330.0, 40.0),
        };

        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(height).at_most(800.0))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.empty();
                strip.cell(|ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        style_ui(ui, Style::SelectButton);
                        StripBuilder::new(ui)
                            .size(Size::remainder())
                            .sizes(Size::exact(row_height), rows)
                            .size(Size::remainder())
                            .vertical(|mut strip| {
                                strip.empty();
                                for row in 0..rows {
                                    strip.strip(|mut builder| {
                                        let this_cols = if row < rows - 1 || names.len() % cols == 0
                                        {
                                            cols
                                        } else {
                                            names.len() % cols
                                        };

                                        builder = builder
                                            .size(Size::remainder())
                                            .size(Size::exact(col_width));
                                        for _ in 1..this_cols {
                                            builder = builder
                                                .size(Size::exact(col_spacing))
                                                .size(Size::exact(col_width));
                                        }
                                        builder = builder.size(Size::remainder());

                                        builder.horizontal(|mut strip| {
                                            for j in 0..this_cols {
                                                let which = row * cols + j;
                                                strip.empty();
                                                strip.cell(|ui| {
                                                    ui.centered_and_justified(|ui| {
                                                        let (style, hint) = match is_done[which] {
                                                            Progress::None => {
                                                                (Style::TodoButton, None)
                                                            }
                                                            Progress::Success(t) => (
                                                                Style::DoneButton,
                                                                Some(tooltip(format!(
                                                                    "Completed @ {}",
                                                                    t.format("%Y-%m-%d %H:%M:%S")
                                                                ))),
                                                            ),
                                                            Progress::Interrupt(t) => (
                                                                Style::InterruptedButton,
                                                                Some(tooltip(format!(
                                                                    "Interrupted @ {}",
                                                                    t.format("%Y-%m-%d %H:%M:%S")
                                                                ))),
                                                            ),
                                                            Progress::Failure(t, e) => (
                                                                Style::FailedButton,
                                                                Some(tooltip(format!(
                                                                    "Failed @ {}\n\n{e}",
                                                                    t.format("%Y-%m-%d %H:%M:%S")
                                                                ))),
                                                            ),
                                                            Progress::CleanupError(t, e) => (
                                                                Style::SoftFailedButton,
                                                                Some(tooltip(format!(
                                                                    "Failed in clean-up @ {}\n\n{e}",
                                                                    t.format("%Y-%m-%d %H:%M:%S")
                                                                ))),
                                                            ),
                                                            Progress::LastRun(t) => (
                                                                Style::TodoButton,
                                                                Some(tooltip(format!(
                                                                    "Last run @ {}",
                                                                    t.format("%Y-%m-%d %H:%M:%S")
                                                                ))),
                                                            ),
                                                        };

                                                        style_ui(ui, style);
                                                        let response = if let Some(hint) = hint {
                                                            ui.button(&names[which])
                                                                .on_hover_text(hint)
                                                        } else {
                                                            ui.button(&names[which])
                                                        };

                                                        if response.clicked() {
                                                            interaction =
                                                                Interaction::StartBlock(which);
                                                        }
                                                    });
                                                });
                                            }
                                            strip.empty();
                                        });
                                    });
                                }
                                strip.empty();
                            });
                    });
                });
                strip.empty();
            });

        match interaction {
            Interaction::None => {}
            Interaction::StartBlock(i) => {
                if self.scheduler.is_none() {
                    println!("\nStarting experiment block {i}...");
                    self.active_block = Some(i);
                    self.page = Page::Loading;

                    let env = self.env().clone();
                    let block = self.task.block(i);
                    let config = block.config(self.task.config());
                    let resources = block.resources(&config);
                    let mut sync_writer = self.sync_reader.writer();
                    let mut resource_map = self.resources().clone();
                    let tex_manager = ui.ctx().tex_manager();
                    thread::spawn(move || {
                        match resource_map.preload_block(resources, tex_manager, &config, &env) {
                            Ok(()) => sync_writer.push(ServerSignal::LoadComplete),
                            Err(e) => sync_writer.push(ServerSignal::BlockCrashed(e)),
                        }
                    });
                }
            }
        }
    }

    fn show_selection_controls(&mut self, builder: StripBuilder) {
        enum Interaction {
            None,
            Back,
        }

        let mut interaction = Interaction::None;

        center_x(builder, 200.0, |ui| {
            ui.horizontal_centered(|ui| {
                style_ui(ui, Style::CancelButton);
                if ui.button(button1("Back")).clicked() {
                    interaction = Interaction::Back;
                }
            });
        });

        match interaction {
            Interaction::None => {}
            Interaction::Back => self.page = Page::Startup,
        }
    }

    fn show_selection_status(&mut self, ctx: &egui::Context) {
        let header = body(self.active_block.map_or("", |i| &self.blocks[i].0)).strong();
        let content = match &self.status {
            Progress::None => None,
            Progress::Success(_) => Some(body("Block completed successfully!").color(FOREST_GREEN)),
            Progress::Interrupt(_) => {
                Some(body("Block was interrupted by user.").color(CUSTOM_BLUE))
            }
            Progress::Failure(_, e) => {
                Some(body(format!("\nError in block execution:\n\n{e}\n")).color(CUSTOM_RED))
            }
            Progress::CleanupError(_, e) => Some(
                body(format!("\nFailed to clean up after block:\n\n{e}\n")).color(CUSTOM_ORANGE),
            ),
            Progress::LastRun(_) => None,
        };

        if let Some(content) = content {
            let mut open = true;

            Window::new(header.size(TEXT_SIZE_DIALOGUE_TITLE))
                .collapsible(false)
                .open(&mut open)
                .vscroll(true)
                .hscroll(false)
                .min_width(920.0)
                .fixed_size(Vec2::new(920.0, 360.0))
                .fixed_pos(Pos2::new(500.0, 280.0))
                .show(ctx, |ui| {
                    ui.with_layout(
                        Layout::centered_and_justified(Direction::LeftToRight),
                        |ui| {
                            ui.add_sized(
                                [760.0, 340.0],
                                Label::new(content.clone().size(TEXT_SIZE_DIALOGUE_BODY)),
                            );
                        },
                    )
                    .response
                    .context_menu(|ui| {
                        if ui
                            .button(RichText::new("Copy").size(TEXT_SIZE_DIALOGUE_BODY))
                            .clicked()
                        {
                            ui.close_menu();
                            ui.output().copied_text = content.text().trim().to_owned();
                        }
                    });
                });

            if !open {
                self.blocks.get_mut(self.active_block.unwrap()).unwrap().1 =
                    std::mem::replace(&mut self.status, Progress::None);
                self.active_block = None;
            }
        }
    }
}
