use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::callback::{CallbackQueue, Destination};
use crate::config::Config;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::{AsyncCallback, SyncCallback};
use crate::style::{style_ui, Style};
use crate::{error, style};
use eframe::egui;
use eframe::egui::CentralPanel;
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    #[serde(default = "defaults::from")]
    from: u32,
    #[serde(default)]
    style: String,
}

mod defaults {
    #[inline(always)]
    pub fn from() -> u32 {
        3
    }
}

impl From<u32> for Counter {
    fn from(i: u32) -> Self {
        Self {
            from: i,
            style: "".to_owned(),
        }
    }
}

impl Action for Counter {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    fn stateful(
        &self,
        id: usize,
        _res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Ok(Box::new(StatefulCounter {
            id,
            done: false,
            count: self.from,
            // style: Style::new("action-counter", &self.style),
        }))
    }
}

#[derive(Debug)]
pub struct StatefulCounter {
    id: usize,
    done: bool,
    count: u32,
}

impl StatefulAction for StatefulCounter {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(self.done || self.count == 0)
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        false
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        sync_queue: &mut CallbackQueue<SyncCallback>,
        _async_queue: &mut CallbackQueue<AsyncCallback>,
    ) -> Result<(), error::Error> {
        enum Interaction {
            None,
            Decrement,
        };

        let mut interaction = Interaction::None;

        let button = egui::Button::new(format!("Click me {} more times", self.count));

        CentralPanel::default().show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::exact(200.0))
                .size(Size::remainder())
                .horizontal(|mut strip| {
                    strip.empty();
                    strip.strip(|builder| {
                        builder
                            .size(Size::remainder())
                            .size(Size::exact(40.0))
                            .size(Size::remainder())
                            .vertical(|mut strip| {
                                strip.empty();
                                strip.cell(|ui| {
                                    ui.centered_and_justified(|ui| {
                                        style_ui(ui, Style::SelectButton);
                                        if ui.add(button).clicked() {
                                            interaction = Interaction::Decrement;
                                        }
                                    });
                                });
                                strip.empty();
                            });
                    });
                    strip.empty();
                });
        });

        match interaction {
            Interaction::None => {}
            Interaction::Decrement => {
                self.count = self.count.saturating_sub(1);
                if self.count == 0 {
                    println!("Counter done!");
                    self.done = true;
                    sync_queue.push(Destination::default(), SyncCallback::UpdateGraph);
                } else {
                    println!("{} more to go...", self.count);
                }
            }
        }

        Ok(())
    }
}
