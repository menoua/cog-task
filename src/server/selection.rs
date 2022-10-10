use crate::server::{Page, Server};
use crate::style::{style_ui, Style};
use eframe::egui;
use eframe::egui::{Color32, RichText, ScrollArea, Window};
use egui::CentralPanel;
use egui_extras::{Size, StripBuilder};

impl Server {
    pub(crate) fn show_selection(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::exact(50.0))
                .size(Size::exact(15.0))
                .size(Size::remainder())
                .size(Size::exact(15.0))
                .size(Size::exact(50.0))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(self.task.title())
                                    .color(Color32::BLACK)
                                    .heading(),
                            );
                        });
                    });

                    strip.empty();

                    strip.strip(|builder| {
                        builder
                            .size(Size::remainder())
                            .size(Size::exact(760.0))
                            .size(Size::remainder())
                            .horizontal(|mut strip| {
                                strip.empty();
                                strip.cell(|ui| self.show_selection_blocks(ui));
                                strip.empty();
                            });
                    });

                    strip.empty();

                    strip.strip(|builder| self.show_selection_controls(builder));
                });
        });

        if self.status.is_some() {
            self.show_selection_status(ctx);
        }
    }

    fn show_selection_blocks(&mut self, ui: &mut egui::Ui) {
        enum Interaction {
            None,
            StartBlock(usize),
        }

        let mut interaction = Interaction::None;

        let names = self.task.block_labels();
        let cols = self.config().blocks_per_row() as usize;
        let rows = (names.len() + cols - 1) / cols;
        let row_height = 35.0;
        let height = (row_height + 5.0) * rows as f32 + 10.0;
        let (col_width, col_spacing) = match cols {
            1 => (360.0, 0.0),
            2 => (300.0, 60.0),
            3 => (220.0, 30.0),
            _ => (165.0, 20.0),
        };

        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(height).at_most(400.0))
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
                                                        if ui.button(&names[which]).clicked() {
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
            Interaction::StartBlock(_i) => {
                // TODO
            }
        }
    }

    fn show_selection_controls(&mut self, builder: StripBuilder) {
        enum Interaction {
            None,
            Back,
        }

        let mut interaction = Interaction::None;

        builder
            .size(Size::remainder())
            .size(Size::exact(100.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        style_ui(ui, Style::CancelButton);
                        if ui.button(RichText::new("Back").size(20.0)).clicked() {
                            interaction = Interaction::Back;
                        }
                    });
                });

                strip.empty();
            });

        match interaction {
            Interaction::None => {}
            Interaction::Back => self.page = Page::Startup,
        }
    }

    fn show_selection_status(&mut self, ctx: &egui::Context) {
        let mut open = true;

        match &self.status {
            Some(Ok(status)) => {
                Window::new(
                    RichText::from(format!(
                        "End of block: \"{}\"",
                        self.active_block.map_or("", |i| &self.blocks[i].0)
                    ))
                    .size(14.0)
                    .strong(),
                )
                .collapsible(false)
                .open(&mut open)
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label(RichText::from(status).size(12.0).color(Color32::BLACK));
                });
            }
            Some(Err(status)) => {
                Window::new(
                    RichText::from(format!(
                        "Error in block: \"{}\"",
                        self.active_block.map_or("", |i| &self.blocks[i].0)
                    ))
                    .size(14.0)
                    .strong(),
                )
                .collapsible(false)
                .open(&mut open)
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::from(format!("{:#?}", status))
                            .size(12.0)
                            .color(Color32::RED),
                    );
                });
            }
            None => {}
        }

        if !open {
            self.active_block = None;
            self.status = None;
        }
    }
}
