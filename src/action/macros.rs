macro_rules! include_actions {
    ($($name:ident),* $(,)?) => {
        use crate::action::{Action, StatefulAction};
        use crate::config::Config;
        use crate::error;
        use serde::{Deserialize, Serialize};
        use std::path::Path;

        $(
            pub mod $name;
        )*

        paste::paste! {
            $(
                pub use crate::action::[<$name>]::[<$name:camel>];
            )*
        }

        paste::paste! {
            #[derive(Deserialize, Serialize)]
            #[serde(rename_all = "snake_case")]
            pub enum ActionEnum {
                $(
                    [<$name:camel>]([<$name:camel>]),
                )*
            }

            impl ActionEnum {
                pub fn inner(&self) -> &dyn Action {
                    match self {
                        $(
                            Self::[<$name:camel>](inner) => inner,
                        )*
                    }
                }

                pub fn inner_mut(&mut self) -> &mut dyn Action {
                    match self {
                        $(
                            Self::[<$name:camel>](inner) => inner,
                        )*
                    }
                }

                pub fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
                    match self {
                        $(
                            Self::[<$name:camel>](inner) => inner.init(root_dir, config),
                        )*
                    }
                }
            }

            $(
                impl From<[<$name:camel>]> for ActionEnum {
                    fn from(f: [<$name:camel>]) -> ActionEnum {
                        ActionEnum::[<$name:camel>](f)
                    }
                }
            )*

            impl std::fmt::Debug for ActionEnum {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{:#?}", self.inner())
                }
            }
        }
    }
}

macro_rules! include_stateful_actions {
    ($($name:ident),* $(,)?) => {
        paste::paste!(
            $(
                pub use crate::action::[<$name>]::[<Stateful $name:camel>];
            )*
        );

        paste::paste! {
            pub enum StatefulActionEnum {
                $(
                    [<$name:camel>]([<Stateful $name:camel>]),
                )*
            }

            impl StatefulActionEnum {
                pub fn inner(&self) -> &dyn StatefulAction {
                    match self {
                        $(
                            Self::[<$name:camel>](inner) => inner,
                        )*
                    }
                }

                pub fn inner_mut(&mut self) -> &mut dyn StatefulAction {
                    match self {
                        $(
                            Self::[<$name:camel>](inner) => inner,
                        )*
                    }
                }
            }

            impl std::fmt::Debug for StatefulActionEnum {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    use itertools::Itertools;

                    write!(
                        f,
                        "Action({})",
                        self.inner()
                            .debug()
                            .iter()
                            .map(|(key, value)| format!("{key}={value}"))
                            .join(", ")
                    )
                }
            }

            $(
                impl From<[<Stateful $name:camel>]> for StatefulActionEnum {
                    fn from(f: [<Stateful $name:camel>]) -> StatefulActionEnum {
                        StatefulActionEnum::[<$name:camel>](f)
                    }
                }
            )*
        }
    }
}

macro_rules! impl_base_stateful {
    ($name:ident) => {
        paste::paste! {
            impl [<Stateful $name>] {
                #[inline(always)]
                fn id(&self) -> usize {
                    self.id
                }

                #[inline(always)]
                fn type_str(&self) -> String {
                    String::from(stringify!([<$name:snake>]))
                }
            }

            use crate::action::ImplStatefulAction;
            impl ImplStatefulAction for [<Stateful $name>] {}
        }
    };
}

macro_rules! stateful {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        paste::paste! {
            pub struct [<Stateful $name>] {
                id: usize,
                done: bool,
                $(
                    $field: $ty,
                )*
            }

            impl_base_stateful!($name);

            impl [<Stateful $name>] {
                #[inline(always)]
                fn is_over(&self) -> Result<bool, error::Error> {
                    Ok(self.done)
                }
            }
        }
    }
}

macro_rules! stateful_arc {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        paste::paste! {
            pub struct [<Stateful $name>] {
                id: usize,
                done: Arc<Mutex<Result<bool, error::Error>>>,
                $(
                    $field: $ty,
                )*
            }

            impl_base_stateful!($name);

            impl [<Stateful $name>] {
                #[inline(always)]
                fn is_over(&self) -> Result<bool, error::Error> {
                    self.done.lock().unwrap().clone()
                }
            }
        }
    }
}

macro_rules! impl_stateful {
    () => {
        #[inline(always)]
        fn id(&self) -> usize {
            self.id
        }

        #[inline(always)]
        fn is_over(&self) -> Result<bool, error::Error> {
            self.is_over()
        }

        #[inline(always)]
        fn type_str(&self) -> String {
            self.type_str()
        }
    };
}
