use crate::error;
use crate::error::Error::TaskDefinitionError;
use crate::scheduler::graph::Edge;
use itertools::Itertools;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::slice::Iter;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
enum Id {
    Name(String),
    Index(usize),
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdVisitor;
        impl<'de> Visitor<'de> for IdVisitor {
            type Value = Id;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a unit or a string")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Id::Index(v as usize))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Id::Name(v.to_string()))
            }
        }

        deserializer.deserialize_any(IdVisitor)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdList(Vec<Id>);

impl IdList {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn normalize(&mut self, id_map: &HashMap<&String, usize>) -> Result<(), error::Error> {
        for id in self.0.iter_mut() {
            if let Id::Name(n) = id {
                *id = Id::Index(*id_map.get(n).ok_or_else(|| {
                    TaskDefinitionError(format!(
                        "Undefined label `{n}` referenced in flow definition."
                    ))
                })?);
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn vec(&self) -> Vec<Id> {
        self.0.clone()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum FlowConnection {
    Origin {
        target: IdList,
    },
    Starter {
        condition: IdList,
        target: IdList,
    },
    Stopper {
        condition: IdList,
        target: IdList,
    },
    TimedStarter {
        #[serde(deserialize_with = "duration_from_float")]
        duration: Duration,
        target: IdList,
    },
    TimedStopper {
        #[serde(deserialize_with = "duration_from_float")]
        duration: Duration,
        target: IdList,
    },
}

impl FlowConnection {
    fn edge_count(&self) -> usize {
        match self {
            FlowConnection::Starter {
                condition, target, ..
            } => condition.len() * target.len(),
            FlowConnection::Stopper {
                condition, target, ..
            } => condition.len() * target.len(),
            _ => 0,
        }
    }

    fn normalize(&mut self, id_map: &HashMap<&String, usize>) -> Result<(), error::Error> {
        for (&label, &id) in id_map.iter() {
            if label.is_empty() {
                Err(TaskDefinitionError(format!(
                    "Action node `{id}`'s name cannot be an empty string"
                )))?;
            }
            if label.contains(' ') || label.contains('\t') {
                Err(TaskDefinitionError(format!(
                    "Action node `{id}`'s name (`{label}`) cannot contain whitespace"
                )))?;
            }
            if "0123456789".contains(&label[0..1]) {
                Err(TaskDefinitionError(format!(
                    "Action node `{id}`'s name (`{label}`) cannot start with a digit"
                )))?;
            }
        }

        match self {
            FlowConnection::Origin { target } => {
                target.normalize(id_map)?;
            }
            FlowConnection::Starter { condition, target } => {
                condition.normalize(id_map)?;
                target.normalize(id_map)?;
            }
            FlowConnection::Stopper { condition, target } => {
                condition.normalize(id_map)?;
                target.normalize(id_map)?;
            }
            FlowConnection::TimedStarter { target, .. } => {
                target.normalize(id_map)?;
            }
            FlowConnection::TimedStopper { target, .. } => {
                target.normalize(id_map)?;
            }
        }

        Ok(())
    }
}

fn duration_from_float<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = f64::deserialize(deserializer)?;
    let ms = (s * 1000.0).round() as u64;
    Ok(Duration::from_millis(ms))
}

pub enum Timer {
    StartTimer(Duration),
    StopTimer(Duration),
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Flow(Vec<FlowConnection>);

impl Flow {
    pub fn fallback(&mut self, n: usize) -> bool {
        if !self.0.is_empty() || n == 0 {
            return false;
        }

        self.0.clear();
        self.0.push(FlowConnection::Origin {
            target: IdList(vec![Id::Index(0)]),
        });
        for i in 0..(n - 1) {
            self.0.push(FlowConnection::Starter {
                condition: IdList(vec![Id::Index(i)]),
                target: IdList(vec![Id::Index(i + 1)]),
            });
        }
        true
    }

    pub fn iter(&self) -> Iter<FlowConnection> {
        self.0.iter()
    }

    pub fn edge_count(&self) -> usize {
        self.0.iter().map(|c| c.edge_count()).sum::<usize>()
    }

    pub fn normalize(&mut self, id_map: HashMap<&String, usize>) -> Result<(), error::Error> {
        for c in self.0.iter_mut() {
            c.normalize(&id_map)?;
        }
        Ok(())
    }

    pub fn origin(&self) -> Vec<usize> {
        self.0
            .iter()
            .flat_map(|c| match c {
                FlowConnection::Origin { target } => target
                    .vec()
                    .into_iter()
                    .map(|v| match v {
                        Id::Index(v) => v,
                        _ => panic!(),
                    })
                    .collect(),
                _ => vec![],
            })
            .collect()
    }

    pub fn edges(&self) -> Vec<(usize, usize, Edge)> {
        self.0
            .iter()
            .flat_map(|c| match c {
                FlowConnection::Starter { condition, target } => condition
                    .vec()
                    .into_iter()
                    .cartesian_product(target.vec())
                    .map(|(v, w)| match (v, w) {
                        (Id::Index(v), Id::Index(w)) => (v, w, Edge::Starter),
                        _ => panic!(),
                    })
                    .collect(),
                FlowConnection::Stopper { condition, target } => condition
                    .vec()
                    .into_iter()
                    .cartesian_product(target.vec())
                    .map(|(v, w)| match (v, w) {
                        (Id::Index(v), Id::Index(w)) => (v, w, Edge::Stopper),
                        _ => panic!(),
                    })
                    .collect(),
                _ => vec![],
            })
            .collect()
    }

    pub fn timers(&self) -> Vec<(usize, Timer)> {
        self.0
            .iter()
            .flat_map(|c| match c {
                FlowConnection::TimedStarter { duration, target } => target
                    .vec()
                    .into_iter()
                    .map(|v| match v {
                        Id::Index(v) => (v, Timer::StartTimer(*duration)),
                        _ => panic!(),
                    })
                    .collect(),
                FlowConnection::TimedStopper { duration, target } => target
                    .vec()
                    .into_iter()
                    .map(|v| match v {
                        Id::Index(v) => (v, Timer::StopTimer(*duration)),
                        _ => panic!(),
                    })
                    .collect(),
                _ => vec![],
            })
            .collect()
    }
}
