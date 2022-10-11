use crate::action::{ExtAction, StatefulAction};
use crate::callback::CallbackQueue;
use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{FlowError, TaskDefinitionError};
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::flow::{Flow, Timer};
use crate::scheduler::{AsyncCallback, SyncCallback};
use petgraph::prelude::{NodeIndex, StableGraph};
use petgraph::stable_graph::{Edges, NodeIndices};
use petgraph::{Directed, Direction, EdgeDirection};
use std::time::Duration;

#[derive(Debug)]
pub struct Node {
    name: Option<String>,
    pub log_when: LogCondition,
    pub start_timer: Option<Duration>,
    pub stop_timer: Option<Duration>,
    pub action: Box<dyn StatefulAction>,
}

impl Node {
    fn new(
        id: usize,
        action: &ExtAction,
        res: &ResourceMap,
        config: &Config,
        io: &IO,
    ) -> Result<Self, error::Error> {
        Ok(Self {
            name: action.id().map(|s| s.to_owned()),
            log_when: action.log_when().unwrap_or(config.log_when()),
            start_timer: None,
            stop_timer: None,
            action: action.inner().stateful(id, res, config, io)?,
        })
    }

    #[inline(always)]
    pub fn start(
        &mut self,
        sync_queue: &mut CallbackQueue<SyncCallback>,
        async_queue: &mut CallbackQueue<AsyncCallback>,
    ) -> Result<(), error::Error> {
        self.action.start(sync_queue, async_queue)
    }

    #[inline(always)]
    pub fn name(&self) -> &Option<String> {
        &self.name
    }
}

#[derive(Debug)]
pub enum Edge {
    Starter,
    Stopper,
}

#[derive(Debug)]
pub struct DependencyGraph(StableGraph<Node, Edge, Directed, usize>);

impl DependencyGraph {
    pub fn new(
        actions: &[ExtAction],
        flow: &Flow,
        resources: &ResourceMap,
        config: &Config,
        io: &IO,
    ) -> Result<(Self, Vec<NodeIndex<usize>>), error::Error> {
        let mut graph = StableGraph::with_capacity(actions.len(), flow.edge_count());
        let mut nodes = vec![];

        for (i, action) in actions.iter().enumerate() {
            let node = graph.add_node(Node::new(i, action, resources, config, io)?);
            nodes.push(node);
        }

        flow.edges().into_iter().for_each(|(v, w, e)| {
            graph.add_edge(nodes[v], nodes[w], e);
        });

        for (v, t) in flow.timers() {
            if let Some(v) = graph.node_weight_mut(nodes[v]) {
                match t {
                    Timer::StartTimer(dur) => v.start_timer = Some(dur),
                    Timer::StopTimer(dur) => v.stop_timer = Some(dur),
                }
            } else {
                Err(TaskDefinitionError(format!(
                    "Out of bounds index `{v}` referenced in flow."
                )))?;
            }
        }

        let graph = Self(graph);
        graph.verify(&flow.origin())?;

        Ok((graph, nodes))
    }

    fn verify(&self, origin: &[usize]) -> Result<(), error::Error> {
        for v in self.0.node_indices() {
            let node = self.0.node_weight(v).unwrap();
            let mut starter_edge = false;
            let mut stopper_edge = false;
            self.0
                .edges_directed(v, EdgeDirection::Incoming)
                .for_each(|e| match e.weight() {
                    Edge::Starter => starter_edge = true,
                    Edge::Stopper => stopper_edge = true,
                });
            let origin = origin.contains(&v.index());

            if origin && (starter_edge || node.start_timer.is_some()) {
                Err(FlowError(format!(
                    "Origin node `{}` cannot have a start condition",
                    v.index()
                )))?;
            }
            if starter_edge && node.start_timer.is_some() {
                Err(FlowError(format!(
                    "Node `{}` cannot start with both a flow connection AND a timer",
                    v.index()
                )))?;
            }
            if stopper_edge && node.stop_timer.is_some() {
                Err(FlowError(format!(
                    "Node `{}` cannot stop with both a flow connection AND a timer",
                    v.index()
                )))?;
            }
            if node.action.is_static() && !stopper_edge && node.stop_timer.is_none() {
                Err(FlowError(format!(
                    "Static node `{}` needs a stopping condition as it cannot end on its own",
                    v.index()
                )))?;
            }
            if !origin && !starter_edge && node.start_timer.is_none() {
                Err(FlowError(format!("Node `{}` is unreachable", v.index())))?;
            }
        }

        if petgraph::algo::is_cyclic_directed(&self.0) {
            Err(FlowError(
                "Flow graph contains a cycle, and so invalid.".to_owned(),
            ))?;
        }

        Ok(())
    }

    pub fn node(&self, v: NodeIndex<usize>) -> Option<&Node> {
        self.0.node_weight(v)
    }

    pub fn node_mut(&mut self, v: NodeIndex<usize>) -> Option<&mut Node> {
        self.0.node_weight_mut(v)
    }

    #[inline(always)]
    pub fn node_count(&self) -> usize {
        self.0.node_count()
    }

    #[inline(always)]
    pub fn node_indices(&self) -> NodeIndices<Node, usize> {
        self.0.node_indices()
    }

    #[inline(always)]
    pub fn edges(&self, v: NodeIndex<usize>) -> Edges<Edge, Directed, usize> {
        self.0.edges(v)
    }

    #[inline(always)]
    pub fn edges_directed(
        &self,
        v: NodeIndex<usize>,
        dir: Direction,
    ) -> Edges<Edge, Directed, usize> {
        self.0.edges_directed(v, dir)
    }

    #[inline(always)]
    pub fn contains_node(&self, v: NodeIndex<usize>) -> bool {
        self.0.contains_node(v)
    }

    #[inline(always)]
    pub fn remove_node(&mut self, v: NodeIndex<usize>) -> Option<Node> {
        self.0.remove_node(v)
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}
