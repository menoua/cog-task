use crate::action::nil::StatefulNil;
use crate::action::resource::ResourceMap;
use crate::action::{Action, ActionSignal};
use crate::comm::{QReader, QWriter, Signal, MAX_QUEUE_SIZE};
use crate::server::{AsyncSignal, Atomic, Config, LoggerSignal, ServerSignal, State, IO};
use eframe::egui;
use eyre::{eyre, Result};
use serde_cbor::Value;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum SyncSignal {
    UpdateGraph,
    KeyPress(Instant, HashSet<egui::Key>),
    Emit(Instant, Signal),
    Repaint,
    Finish,
}

pub struct SyncProcessor {
    ctx: egui::Context,
    atomic: Atomic,
    sync_reader: QReader<SyncSignal>,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

impl PartialEq for SyncSignal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SyncSignal::UpdateGraph, SyncSignal::UpdateGraph) => true,
            (SyncSignal::KeyPress(t1, _), SyncSignal::KeyPress(t2, _)) => t1 == t2,
            (SyncSignal::Emit(_, _), SyncSignal::Emit(_, _)) => false,
            (SyncSignal::Repaint, SyncSignal::Repaint) => true,
            (SyncSignal::Finish, SyncSignal::Finish) => true,
            _ => false,
        }
    }
}

impl Eq for SyncSignal {}

impl From<Signal> for SyncSignal {
    fn from(signal: Signal) -> Self {
        SyncSignal::Emit(Instant::now(), signal)
    }
}

impl SyncProcessor {
    pub fn spawn(
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        ctx: &egui::Context,
        tree: &dyn Action,
        async_writer: &QWriter<AsyncSignal>,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<(QWriter<SyncSignal>, Atomic)> {
        let sync_reader = QReader::new();
        let sync_writer = sync_reader.writer();
        let tree = tree.stateful(io, res, config, &sync_writer, async_writer)?;
        let atomic = Arc::new(Mutex::new((tree, State::new())));
        let mut proc = Self {
            ctx: ctx.clone(),
            atomic,
            sync_reader,
            sync_writer,
            async_writer: async_writer.clone(),
            server_writer: server_writer.clone(),
        };

        let sync_writer = proc.sync_writer.clone();
        let atomic = proc.atomic.clone();

        thread::spawn(move || {
            proc.sync_reader.pop();
            proc.start().unwrap_or_else(|e| {
                proc.server_writer.push(ServerSignal::BlockCrashed(e));
            });

            'mainloop: while let Ok(signals) = proc.sync_reader.poll() {
                let mut n_signal = signals.len();
                let mut signals = VecDeque::from(signals);
                while let Some(signal) = signals.pop_front() {
                    let new_signals = match signal {
                        SyncSignal::UpdateGraph => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();
                            tree.update(
                                &ActionSignal::UpdateGraph,
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                        }
                        SyncSignal::KeyPress(time, keys) => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();
                            tree.update(
                                &ActionSignal::KeyPress(time, keys),
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                        }
                        SyncSignal::Emit(time, signal) => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();

                            let mut changed = HashSet::new();
                            if !signal.is_empty() {
                                for (k, v) in signal.into_iter() {
                                    state.insert(k, v);
                                    changed.insert(k);
                                }
                            }

                            tree.update(
                                &ActionSignal::StateChanged(time, changed),
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                        }
                        SyncSignal::Repaint => {
                            proc.ctx.request_repaint();
                            Ok(vec![])
                        }
                        SyncSignal::Finish => break 'mainloop,
                    }
                    .unwrap_or_else(|e| {
                        proc.server_writer.push(ServerSignal::BlockCrashed(e));
                        proc.ctx.request_repaint();
                        vec![]
                    });

                    if !new_signals.is_empty() {
                        n_signal += new_signals.len();
                        if n_signal > MAX_QUEUE_SIZE {
                            proc.server_writer.push(ServerSignal::BlockCrashed(eyre!(
                                "Number of signals in a single poll exceeded MAX_QUEUE_SIZE."
                            )));
                            proc.ctx.request_repaint();
                        } else {
                            for signal in new_signals.into_iter().rev() {
                                if let SyncSignal::Emit(_, _) = signal {
                                    signals.push_front(signal);
                                } else {
                                    proc.server_writer.push(ServerSignal::BlockCrashed(eyre!(
                                        "Action sent a non-emit signal which is not allowed."
                                    )));
                                    proc.ctx.request_repaint();
                                }
                            }
                        }
                    }

                    let (tree, state) = &mut *proc.atomic.lock().unwrap();
                    let is_over = tree.is_over().unwrap_or_else(|e| {
                        proc.server_writer.push(ServerSignal::BlockCrashed(e));
                        let _ = tree.stop(&mut proc.sync_writer, &mut proc.async_writer, state);
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                        false
                    });

                    if is_over {
                        proc.server_writer.push(ServerSignal::BlockFinished);
                        tree.stop(&mut proc.sync_writer, &mut proc.async_writer, state)?;
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                    }
                }
            }

            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
            proc.ctx.request_repaint();
            Result::<()>::Ok(())
        });

        Ok((sync_writer, atomic))
    }

    fn start(&mut self) -> Result<()> {
        let (tree, state) = &mut *self.atomic.lock().unwrap();

        self.async_writer.push(LoggerSignal::Append(
            "main".to_owned(),
            ("start".to_owned(), Value::Text("ok".to_owned())),
        ));

        tree.start(&mut self.sync_writer, &mut self.async_writer, state)?;

        if tree.is_over()? {
            self.server_writer.push(ServerSignal::BlockFinished);
            self.ctx.request_repaint();
        }

        Ok(())
    }
}
