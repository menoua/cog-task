use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{ActionViewError, InvalidNameError};
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::monitor::{Event, Monitor};
use crate::scheduler::{AsyncCallback, SyncCallback};
use eframe::egui;
use gstreamer::paste;
use itertools::Itertools;
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};

#[macro_use]
mod macros;
pub mod include;
use crate::signal::QWriter;
pub use include::*;

pub trait Action: Debug {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    #[inline(always)]
    fn init(&self) -> Result<(), error::Error> {
        Ok(())
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        config: &Config,
        io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error>;

    #[inline(always)]
    fn evolve(
        &self,
        _root_dir: &Path,
        _config: &Config,
    ) -> Result<Option<Box<dyn Action>>, error::Error> {
        Ok(None)
    }
}

pub trait StatefulAction: Send {
    fn id(&self) -> usize;

    fn is_over(&self) -> Result<bool, error::Error>;

    fn type_str(&self) -> String;

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        true
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        None
    }

    fn stop(&mut self) -> Result<(), error::Error>;

    #[inline(always)]
    fn start(
        &mut self,
        sync_queue: &mut QWriter<SyncCallback>,
        async_queue: &mut QWriter<AsyncCallback>,
    ) -> Result<(), error::Error> {
        Ok(())
    }

    #[inline(always)]
    fn update(
        &mut self,
        callback: ActionCallback,
        sync_queue: &mut QWriter<SyncCallback>,
        async_queue: &mut QWriter<AsyncCallback>,
    ) -> Result<(), error::Error> {
        Ok(())
    }

    #[inline(always)]
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_queue: &mut QWriter<SyncCallback>,
        async_queue: &mut QWriter<AsyncCallback>,
    ) -> Result<(), error::Error> {
        Err(ActionViewError(format!(
            "Attempted to show a non-visual action: Action({})",
            self.debug()
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .join(", ")
        )))
    }

    fn debug(&self) -> Vec<(&str, String)> {
        vec![
            ("id", format!("{:?}", self.id())),
            ("over", format!("{:?}", self.is_over())),
            ("visual", format!("{:?}", self.is_visual())),
            ("static", format!("{:?}", self.is_static())),
            ("monitors", format!("{:?}", self.monitors())),
            ("type", format!("{:?}", self.type_str())),
        ]
    }
}

pub trait ImplStatefulAction: StatefulAction {}

impl Debug for dyn StatefulAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Action({})",
            self.debug()
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .join(", ")
        )
    }
}

#[derive(Debug, Clone)]
pub enum StatefulActionMsg {
    Update(usize),
    UpdateInt(usize, i32),
    UpdateUInt(usize, u32),
    UpdateFloat(usize, f32),
    UpdateBool(usize, bool),
    UpdateString(usize, String),
    UpdateEvent(Event),
}

#[derive(Debug)]
pub struct ExtAction {
    id: Option<String>,
    action: Box<dyn Action>,
    log_when: Option<LogCondition>,
}

impl ExtAction {
    pub fn new(
        id: Option<String>,
        action: Box<dyn Action>,
        log_when: Option<LogCondition>,
    ) -> Self {
        Self {
            id,
            action,
            log_when,
        }
    }

    #[inline(always)]
    pub fn id(&self) -> Option<&String> {
        self.id.as_ref()
    }

    #[inline(always)]
    pub fn inner(&self) -> &dyn Action {
        self.action.as_ref()
    }

    #[inline(always)]
    pub fn log_when(&self) -> &Option<LogCondition> {
        &self.log_when
    }

    pub fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        if let Some(id) = &self.id {
            ExtAction::verify_id(id)?;
        }
        if let Some(descendent) = self.action.evolve(root_dir, config)? {
            self.action = descendent;
        }
        self.action.init()?;
        Ok(())
    }

    fn verify_id(id: &str) -> Result<(), error::Error> {
        if id.is_empty() {
            Err(InvalidNameError(
                "Action `id` cannot be the empty string. You can use `None` to ignore the name."
                    .to_owned(),
            ))
        } else if id.split_whitespace().count() > 1 {
            Err(InvalidNameError(format!(
                "Action `id` cannot contain whitespaces: '{id}'"
            )))
        } else if !id.chars().next().unwrap().is_alphabetic() {
            Err(InvalidNameError(format!(
                "Action `id` cannot start with a non-alphabetic character: '{id}'"
            )))
        } else if !id
            .chars()
            .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "-_".contains(c))
        {
            Err(InvalidNameError(format!(
                "Action `id` characters need to be alphanumeric or one of '-' or '_': '{id}'"
            )))
        } else {
            Ok(())
        }
    }
}

// #[derive(Debug)]
// struct Style(Vec<String>);
// impl Style {
//     fn new(base: &str, custom: &str) -> Self {
//         Self(
//             format!("{base} {custom}")
//                 .split_whitespace()
//                 .map(|c| c.to_owned())
//                 .collect(),
//         )
//     }
// }

#[derive(Debug, Clone)]
pub enum ActionCallback {
    KeyPress(HashSet<egui::Key>),
}

mod de {
    use super::{include, Action, ExtAction};
    use crate::config::LogCondition;
    use serde::de::{Error, MapAccess, Visitor};
    use serde::{Deserialize, Serialize, Serializer};
    use serde_json::value::{Map, Value};
    use std::fmt;

    impl<'de> Deserialize<'de> for ExtAction {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_map(ExtActionVisitor)
        }
    }

    struct ExtActionVisitor;
    impl<'de> Visitor<'de> for ExtActionVisitor {
        type Value = ExtAction;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A bool, i64, f64, or String")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut fields = vec![];
            while let Some(entry) = map.next_entry()? {
                fields.push(entry);
            }
            let mut fields: Map<String, Value> = fields.into_iter().collect();

            if !fields.contains_key("type") {
                return Err(Error::custom("Action definition is missing a `type`"));
            }

            let id = match fields.get("id") {
                Some(Value::String(s)) => Some(s.to_owned()),
                Some(_) => {
                    return Err(Error::custom(
                        "Action `id` should be a string or None/ignored",
                    ));
                }
                _ => None,
            };
            let action_type = match fields.get("type") {
                Some(Value::String(s)) => s.to_owned(),
                _ => {
                    return Err(Error::custom("Action `type` should be a string"));
                }
            };
            let log_when = if let Some(v) = fields.get("log_when") {
                Some(
                    serde_json::from_value::<LogCondition>(v.clone()).map_err(|e| {
                        Error::custom(format!("Failed to interpret value for `log_when`:\n{e:#?}"))
                    })?,
                )
            } else {
                None
            };
            fields.retain(|k, _| !["type", "id", "log_when"].contains(&k.as_str()));

            let fields =
                serde_json::to_vec(&fields).map_err(|e| Error::custom(format!("{e:#?}")))?;

            let action = include::from_name_and_fields(&action_type, fields)
                .map_err(|e| Error::custom(format!("{e:#?}")))?;

            if let Some(action) = action {
                Ok(ExtAction::new(id, action, log_when))
            } else {
                Err(Error::custom(format!("Unknown action type: {action_type}")))
            }
        }
    }

    impl Serialize for ExtAction {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&format!("{:?}", self))
        }
    }
}
