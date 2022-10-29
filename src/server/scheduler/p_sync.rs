use crate::action::nil::StatefulNil;
use crate::action::{Action, ActionSignal, StatefulAction};
use crate::comm::{QReader, QWriter, Signal, MAX_QUEUE_SIZE};
use crate::resource::{IoManager, Key, LoggerSignal, ResourceManager};
use crate::server::{AsyncSignal, Atomic, Block, Config, Env, ServerSignal};
use eframe::egui;
use eyre::{eyre, Context, Result};
use serde_cbor::{from_slice, Value};
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum SyncSignal {
    UpdateGraph,
    KeyPress(Instant, BTreeSet<Key>),
    Emit(Instant, Signal),
    Repaint,
    Finish,
    Go,
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
        block: &Block,
        env: &Env,
        config: &Config,
        ctx: &egui::Context,
        async_writer: &QWriter<AsyncSignal>,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<(QWriter<SyncSignal>, Atomic)> {
        let sync_reader = QReader::new();
        let sync_writer = sync_reader.writer();
        let atomic = Arc::new(Mutex::new((
            Box::new(StatefulNil::new()) as Box<dyn StatefulAction>,
            block.default_state().clone(),
        )));
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

        let env = env.clone();
        let config = config.clone();
        let tree = block.action_tree_vec();
        let resources = block.resources(&config);
        let tex_manager = ctx.tex_manager();

        thread::spawn(move || {
            let io_manager = match IoManager::new(&config) {
                Ok(io) => io,
                Err(e) => {
                    proc.server_writer.push(ServerSignal::BlockCrashed(
                        e.wrap_err("Failed to initialize IO manager."),
                    ));
                    proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                    proc.ctx.request_repaint();
                    return;
                }
            };

            let mut res_manager = match ResourceManager::new(&config) {
                Ok(r) => r,
                Err(e) => {
                    proc.server_writer.push(ServerSignal::BlockCrashed(
                        e.wrap_err("Failed to initialize resource manager."),
                    ));
                    proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                    proc.ctx.request_repaint();
                    return;
                }
            };

            if let Err(e) = res_manager.preload_block(resources, tex_manager, &config, &env) {
                proc.server_writer.push(ServerSignal::BlockCrashed(
                    e.wrap_err("Failed to load resources for block."),
                ));
                proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                proc.ctx.request_repaint();
                return;
            }

            let tree = match from_slice::<Box<dyn Action>>(&tree) {
                Ok(tree) => tree,
                Err(e) => {
                    proc.server_writer.push(ServerSignal::BlockCrashed(eyre!(
                        "Failed to transfer action tree to sync process:\n{e:?}"
                    )));
                    proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                    proc.ctx.request_repaint();
                    return;
                }
            };

            let tree = match tree.stateful(
                &io_manager,
                &res_manager,
                &config,
                &proc.sync_writer,
                &proc.async_writer,
            ) {
                Ok(t) => t,
                Err(e) => {
                    proc.server_writer.push(ServerSignal::BlockCrashed(
                        e.wrap_err("Failed to make action tree stateful."),
                    ));
                    proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                    proc.ctx.request_repaint();
                    return;
                }
            };

            proc.server_writer.push(ServerSignal::LoadComplete);
            proc.ctx.request_repaint();

            loop {
                match proc.sync_reader.pop() {
                    Some(SyncSignal::Go) => break,
                    None => return,
                    _ => {}
                }
            }
            thread::sleep(Duration::from_secs(1));

            if let Err(e) = proc.start(tree).wrap_err("Failed to start block.") {
                proc.server_writer.push(ServerSignal::BlockCrashed(e));
                proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                proc.ctx.request_repaint();
                return;
            }

            'mainloop: while let Ok(signals) = proc.sync_reader.poll() {
                let mut n_signal = signals.len();
                let mut signals = VecDeque::from(signals);
                while let Some(signal) = signals.pop_front() {
                    #[cfg(debug_assertions)]
                    println!("{signal:?}");

                    let news = match signal {
                        SyncSignal::UpdateGraph => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();
                            tree.update(
                                &ActionSignal::UpdateGraph,
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                            .wrap_err("Failed to update graph.")
                        }
                        SyncSignal::KeyPress(time, keys) => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();
                            tree.update(
                                &ActionSignal::KeyPress(time, keys),
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                            .wrap_err("Failed to process key press.")
                        }
                        SyncSignal::Emit(time, signal) => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();

                            let mut changed = BTreeSet::new();
                            for (k, v) in signal.into_iter() {
                                state.insert(k, v);
                                changed.insert(k);
                            }

                            tree.update(
                                &ActionSignal::StateChanged(time, changed),
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                            .wrap_err("Failed to emit signal.")
                        }
                        SyncSignal::Repaint => {
                            proc.ctx.request_repaint();
                            Ok(Signal::none())
                        }
                        SyncSignal::Finish => break 'mainloop,
                        SyncSignal::Go => Ok(Signal::none()),
                    };

                    let news = match news {
                        Ok(s) => s,
                        Err(e) => {
                            proc.server_writer.push(ServerSignal::BlockCrashed(e));
                            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                            proc.ctx.request_repaint();
                            return;
                        }
                    };

                    if !news.is_empty() {
                        n_signal += 1;
                        if n_signal > MAX_QUEUE_SIZE {
                            proc.server_writer.push(ServerSignal::BlockCrashed(eyre!(
                                "Number of signals in a single poll exceeded MAX_QUEUE_SIZE."
                            )));
                            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                            proc.ctx.request_repaint();
                            return;
                        } else {
                            signals.push_front(news.into());
                        }
                    }

                    let (tree, state) = &mut *proc.atomic.lock().unwrap();
                    let is_over = tree
                        .is_over()
                        .wrap_err_with(|| eyre!("Failed to check if action over: {tree:?}"));

                    let is_over = match is_over {
                        Ok(c) => c,
                        Err(e) => {
                            proc.server_writer.push(ServerSignal::BlockCrashed(e));
                            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
                            let _ = tree.stop(&mut proc.sync_writer, &mut proc.async_writer, state);
                            *tree = Box::new(StatefulNil::new());
                            proc.ctx.request_repaint();
                            return;
                        }
                    };

                    if is_over {
                        proc.server_writer.push(ServerSignal::BlockFinished);
                        if let Err(e) =
                            tree.stop(&mut proc.sync_writer, &mut proc.async_writer, state)
                        {
                            println!("Failed to graciously finish task:\n{e:?}");
                        }
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                    }
                }
            }

            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
            proc.ctx.request_repaint();
        });

        Ok((sync_writer, atomic))
    }

    fn start(&mut self, root: Box<dyn StatefulAction>) -> Result<()> {
        let (tree, state) = &mut *self.atomic.lock().unwrap();

        self.async_writer.push(LoggerSignal::Append(
            "main".to_owned(),
            ("start".to_owned(), Value::Text("ok".to_owned())),
        ));

        *tree = root;
        let news = tree.start(&mut self.sync_writer, &mut self.async_writer, state)?;
        if !news.is_empty() {
            self.sync_writer.push(SyncSignal::from(news));
        }

        if tree.is_over()? {
            self.server_writer.push(ServerSignal::BlockFinished);
            self.ctx.request_repaint();
        }

        Ok(())
    }
}
