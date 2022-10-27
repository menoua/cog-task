use crate::action::{Action, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::ResourceMap;
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use crate::util::spin_sleeper;
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const MIN_STEP_DELAY: f32 = 0.010;

#[derive(Debug, Deserialize, Serialize)]
pub struct Clock {
    step: f32,
    out_tic: SignalId,
}

stateful!(Clock {
    link: Sender<()>,
});

impl Action for Clock {
    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.out_tic])
    }

    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.step < MIN_STEP_DELAY {
            return Err(eyre!("Step size for clock is smaller than MIN_STEP_DELAY."));
        }

        if self.out_tic == 0 {
            return Err(eyre!("Clock with no `out_signal` is useless."));
        }

        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let (tx, rx) = mpsc::channel();

        {
            let step = Duration::from_secs_f32(self.step);
            let out_tic = self.out_tic;
            let mut sync_writer = sync_writer.clone();

            thread::spawn(move || {
                let sleeper = spin_sleeper();

                if rx.recv().is_err() {
                    return;
                }

                while let Err(TryRecvError::Empty) | Ok(()) = rx.try_recv() {
                    sleeper.sleep(step);
                    sync_writer.push(SyncSignal::Emit(
                        Instant::now(),
                        vec![(out_tic, Value::Null)].into(),
                    ));
                }
            });
        }

        Ok(Box::new(StatefulClock {
            done: false,
            link: tx,
        }))
    }
}

impl StatefulAction for StatefulClock {
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
        self.link
            .send(())
            .wrap_err("Failed to communicate with the clock thread.")?;
        Ok(Signal::none())
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
