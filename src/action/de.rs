use crate::action::question::Question;
use crate::action::video::Video;
use crate::action::{
    Action, Audio, Counter, ExtAction, Fixation, Image, Instruction, KeyLogger, Nop, Simple,
};
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

        let fields = serde_json::to_vec(&fields).map_err(|e| Error::custom(format!("{e:#?}")))?;
        let action = match action_type.as_str() {
            "audio" => Some(boxed::<Audio>(&fields)),
            "counter" => Some(boxed::<Counter>(&fields)),
            "fixation" => Some(boxed::<Fixation>(&fields)),
            "image" => Some(boxed::<Image>(&fields)),
            "instruction" => Some(boxed::<Instruction>(&fields)),
            "key_logger" => Some(boxed::<KeyLogger>(&fields)),
            "nop" => Some(boxed::<Nop>(&fields)),
            "question" => Some(boxed::<Question>(&fields)),
            "simple" => Some(boxed::<Simple>(&fields)),
            "video" => Some(boxed::<Video>(&fields)),
            _ => None,
        };
        let action = match action {
            Some(Ok(action)) => Some(action),
            Some(Err(e)) => Err(Error::custom(format!("{e:#?}")))?,
            None => None,
        };

        if let Some(action) = action {
            Ok(ExtAction::new(id, action, log_when))
        } else {
            Err(Error::custom(format!("Unknown action type: {action_type}")))
        }
    }
}

fn boxed<'de, T: 'static + Action + Deserialize<'de>>(
    fields: &'de [u8],
) -> Result<Box<dyn Action>, serde_json::Error> {
    let action: T = serde_json::from_slice(fields)?;
    Ok(Box::new(action))
}

impl Serialize for ExtAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}
