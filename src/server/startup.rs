use crate::assets::Icon;
use crate::server::{Page, Server};
use crate::style;
use crate::style::{style_ui, Style};
use crate::util::{f32_with_precision, str_with_precision};
use eframe::egui;
use egui::{CentralPanel, Color32, RichText, ScrollArea, TextEdit, Widget};
use egui_extras::{Size, StripBuilder};

impl Server {
    pub(crate) fn show_startup(&mut self, ctx: &egui::Context) {
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
                                strip.cell(|ui| {
                                    ScrollArea::vertical().show(ui, |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(
                                                RichText::new(self.task.description()).size(18.0),
                                            );
                                        });
                                    });
                                });
                                strip.empty();
                            });
                    });

                    strip.empty();

                    strip.strip(|builder| self.show_startup_controls(builder));
                });
        });
    }

    fn show_startup_controls(&mut self, builder: StripBuilder) {
        enum Interaction {
            None,
            Quit,
            ToggleMagnification,
            ZoomIn,
            ZoomOut,
            Start,
        }

        let mut interaction = Interaction::None;

        let builder = builder
            .size(Size::remainder())
            .size(Size::exact(100.0))
            .size(Size::exact(65.0))
            .size(Size::exact(25.0));

        if self.show_magnification {
            builder.size(Size::exact(100.0))
        } else {
            builder
        }
        .size(Size::exact(65.0))
        .size(Size::exact(75.0))
        .size(Size::exact(5.0))
        .size(Size::exact(100.0))
        .size(Size::exact(65.0))
        .size(Size::exact(100.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.empty();

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    style_ui(ui, Style::CancelButton);
                    if ui.button(RichText::new("Quit").size(20.0)).clicked() {
                        interaction = Interaction::Quit;
                    }
                });
            });

            strip.empty();

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    style_ui(ui, Style::IconControls);
                    if ui.button(Icon::MagnifyingGlass.size(16.0)).clicked() {
                        interaction = Interaction::ToggleMagnification;
                    }
                });
            });

            if self.show_magnification {
                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        let response = ui.add(
                            egui::DragValue::new(&mut self.scale_factor)
                                .prefix("x")
                                .clamp_range(0.8..=1.1)
                                .speed(0.01),
                        );

                        self.hold_on_rescale = response.dragged();
                        if response.secondary_clicked() && !response.has_focus() {
                            self.scale_factor = 1.0;
                        }
                    });
                });
            }

            strip.empty();

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Subject ID:").size(18.0));
                });
            });

            strip.empty();

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    TextEdit::singleline(&mut self.subject)
                        .hint_text("Enter Subject ID")
                        .ui(ui);
                });
            });

            strip.empty();

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    style_ui(ui, Style::SubmitButton);
                    let enabled = self.valid_subject_id();
                    ui.add_enabled_ui(enabled, |ui| {
                        if ui.button(RichText::new("Start").size(20.0)).clicked() {
                            interaction = Interaction::Start;
                        }
                    });
                });
            });

            strip.empty();
        });

        match interaction {
            Interaction::None => {}
            Interaction::Quit => std::process::exit(0),
            Interaction::ToggleMagnification => self.show_magnification = !self.show_magnification,
            Interaction::ZoomIn => {
                self.scale_factor = f32_with_precision(self.scale_factor + 0.2, 1).min(1.2);
            }
            Interaction::ZoomOut => {
                self.scale_factor = f32_with_precision(self.scale_factor - 0.2, 1).max(0.8);
            }
            Interaction::Start => {
                self.page = Page::Selection;
                println!("\n{:#?}", self.task.config());
            }
        }
    }
}
