use crate::action::{Action, Props, StatefulAction, VISUAL};
use crate::comm::{QWriter, Signal};
use crate::gui::{style_ui, Style};
use crate::resource::{IoManager, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui;
use egui_extras::{Size, StripBuilder};
use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counter(#[serde(default = "defaults::from")] u32);

stateful!(Counter { count: u32 });

mod defaults {
    #[inline(always)]
    pub fn from() -> u32 {
        3
    }
}

impl From<u32> for Counter {
    fn from(i: u32) -> Self {
        Self(i)
    }
}

impl Action for Counter {
    fn stateful(
        &self,
        _io: &IoManager,
        _res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulCounter {
            done: false,
            count: self.0,
        }))
    }
}

impl StatefulAction for StatefulCounter {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        VISUAL.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        if self.count == 0 {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            sync_writer.push(SyncSignal::Repaint);
        }

        Ok(Signal::none())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        enum Interaction {
            None,
            Decrement,
        }

        let mut interaction = Interaction::None;

        let button = egui::Button::new(format!("Click me {} more times", self.count));

        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(420.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();
                strip.strip(|builder| {
                    builder
                        .size(Size::remainder())
                        .size(Size::exact(80.0))
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

        match interaction {
            Interaction::None => {}
            Interaction::Decrement => {
                self.count = self.count.saturating_sub(1);
                if self.count == 0 {
                    self.done = true;
                    sync_writer.push(SyncSignal::UpdateGraph);
                }
            }
        }

        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("count", format!("{:?}", self.count))])
            .collect()
    }
}
