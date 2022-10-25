use crate::assets::Icon;
use crate::gui::{
    header_body_controls, style_ui, text::body, text::button1, text::inactive, text::tooltip, Style,
};
use crate::server::{Page, Progress, Server};
use chrono::{NaiveDate, NaiveTime};
use eframe::egui;
use egui::{ScrollArea, TextEdit, Widget};
use egui_extras::{Size, StripBuilder};
use eyre::Result;
use heck::ToSnakeCase;

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
                        if ui.button(button1("Quit")).clicked() {
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
                            if ui.button(button1("Start")).clicked() {
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
                for i in 0..self.blocks.len() {
                    if matches!(self.blocks[i].1, Progress::None) {
                        let _ = self.update_history(i);
                    }
                }
                println!("\n{:#?}", self.task.config());
            }
        }
    }

    fn update_history(&mut self, i: usize) -> Result<()> {
        let name = self.blocks[i].0.to_snake_case();
        let progress = &mut self.blocks[i].1;

        let mut last = None;
        let dir = self.env.output().join(&self.subject);
        if let Ok(sessions) = dir.read_dir() {
            for session in sessions {
                let date = session.unwrap().file_name().to_str().unwrap().to_owned();
                let dir = dir.join(&date);
                if let Ok(date) = NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                    if let Ok(runs) = dir.join(&name).read_dir() {
                        for run in runs {
                            let time = run.unwrap().file_name().to_str().unwrap().to_owned();
                            if let Ok(time) = NaiveTime::parse_from_str(&time, "%H-%M-%S") {
                                let datetime = date.and_time(time);

                                match last {
                                    None => last = Some(datetime),
                                    Some(t) if datetime > t => last = Some(datetime),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(t) = last {
            *progress = Progress::LastRun(t);
        }

        Ok(())
    }
}
