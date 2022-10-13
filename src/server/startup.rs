use crate::assets::Icon;
use crate::server::{Page, Server};
use crate::style;
use crate::style::text::{body, heading, inactive, tooltip};
use crate::style::{style_ui, Style, CUSTOM_RED, FOREST_GREEN, TEXT_SIZE_ICON};
use crate::template::header_body_controls;
use crate::util::{f32_with_precision, str_with_precision};
use eframe::egui;
use eframe::egui::style::Margin;
use eframe::egui::{FontSelection, TextStyle};
use eframe::emath::Vec2;
use egui::{CentralPanel, Color32, RichText, ScrollArea, TextEdit, Widget};
use egui_extras::{Size, StripBuilder};

impl Server {
    pub(crate) fn show_startup(&mut self, ui: &mut egui::Ui) {
        header_body_controls(ui, |strip| {
            strip.cell(|ui| {
                ui.centered_and_justified(|ui| ui.heading(self.task.title()));
            });
            strip.empty();
            strip.strip(|builder| {
                builder
                    .size(Size::remainder())
                    .size(Size::exact(1520.0))
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.empty();
                        strip.cell(|ui| {
                            ScrollArea::vertical().show(ui, |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(self.task.description());
                                });
                            });
                        });
                        strip.empty();
                    });
            });
            strip.empty();
            strip.strip(|builder| self.show_startup_controls(builder));
        });
    }

    fn show_startup_controls(&mut self, builder: StripBuilder) {
        enum Interaction {
            None,
            Quit,
            ToggleMagnification,
            Start,
        }

        let mut interaction = Interaction::None;

        builder
            .size(Size::remainder())
            .size(Size::exact(200.0))
            .size(Size::exact(100.0))
            .size(Size::exact(60.0))
            .size(Size::exact(180.0))
            .size(Size::exact(180.0))
            .size(Size::exact(150.0))
            .size(Size::exact(10.0))
            .size(Size::exact(250.0))
            .size(Size::exact(100.0))
            .size(Size::exact(200.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        style_ui(ui, Style::CancelButton);
                        if ui.button(style::text::button1("Quit")).clicked() {
                            interaction = Interaction::Quit;
                        }
                    });
                });

                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        style_ui(ui, Style::IconControls);
                        if ui
                            .button(Icon::MagnifyingGlass)
                            .on_hover_text(tooltip("Magnification"))
                            .clicked()
                        {
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
                                    .clamp_range(0.7..=1.15)
                                    .speed(0.01),
                            );

                            self.hold_on_rescale = response.dragged();
                            if response.secondary_clicked() && !response.has_focus() {
                                self.scale_factor = 1.0;
                            }
                        });
                    });
                } else {
                    strip.empty();
                }

                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        ui.label(body("Subject ID:"));
                    });
                });

                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        style_ui(ui, Style::SingleLineTextEdit);
                        TextEdit::singleline(&mut self.subject)
                            .hint_text(inactive("Enter Subject ID"))
                            .ui(ui);
                    });
                });

                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        style_ui(ui, Style::SubmitButton);
                        let enabled = self.valid_subject_id();
                        ui.add_enabled_ui(enabled, |ui| {
                            if ui.button(style::text::button1("Start")).clicked() {
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
            Interaction::Start => {
                self.page = Page::Selection;
                println!("\n{:#?}", self.task.config());
            }
        }
    }
}
