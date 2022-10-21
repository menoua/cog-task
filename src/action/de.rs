use crate::action::{Action, ActionEnum};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl<'de> Deserialize<'de> for Box<dyn Action> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ActionEnum::deserialize(deserializer)?
            .unwrap()
            .map_err(|e| serde::de::Error::custom(format!("{e:#?}")))
    }
}

impl Serialize for Box<dyn Action> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:#?}", self).serialize(serializer)
    }
}
