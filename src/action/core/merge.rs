use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::ResourceMap;
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize)]
pub struct Merge {
    in_many: BTreeSet<SignalId>,
    out_one: SignalId,
}

stateful!(Merge {
    in_many: BTreeSet<SignalId>,
    out_one: SignalId,
});

impl Action for Merge {
    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.in_many.clone()
    }

    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.out_one])
    }

    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.in_many.is_empty() {
            Err(eyre!("Merge with no inputs is useless."))
        } else if self.out_one == 0 {
            Err(eyre!("Merge without an output is useless."))
        } else if self.in_many.contains(&self.out_one) {
            Err(eyre!("Merge output cannot be connected to its input."))
        } else {
            Ok(Box::new(self))
        }
    }

    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulMerge {
            done: false,
            in_many: self.in_many.clone(),
            out_one: self.out_one,
        }))
    }
}

impl StatefulAction for StatefulMerge {
    impl_stateful!();

    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        Ok(Signal::none())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if let ActionSignal::StateChanged(_, signal) = signal {
            let mut news = BTreeMap::new();
            for id in signal.iter().filter(|&i| self.in_many.contains(i)) {
                if let Some(value) = state.get(id) {
                    news.insert(self.out_one, value.clone());
                }
            }

            Ok(Signal::new(news))
        } else {
            Ok(Signal::none())
        }
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        self.done = true;
        Ok(Signal::none())
    }
}
