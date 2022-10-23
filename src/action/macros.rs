macro_rules! include_actions {
    ($($group:ident::$name:ident),* $(,)?) => {
        use crate::action::Action;
        use crate::error;
        use serde::{Deserialize, Serialize};

        paste::paste! {
            $(
                pub use crate::action::$group::$name;
            )*
        }

        paste::paste! {
            $(
                pub use crate::action::$group::$name::[<$name:camel>];
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
                pub fn unwrap(self) -> Result<Box<dyn Action>, error::Error> {
                    match self {
                        $(
                            Self::[<$name:camel>](inner) => inner.init(),
                        )*
                    }
                }
            }
        }
    }
}

macro_rules! include_stateful_actions {
    ($($group:ident::$name:ident),* $(,)?) => {
        paste::paste!(
            $(
                pub use crate::action::$group::$name::[<Stateful $name:camel>];
            )*
        );
    }
}

macro_rules! impl_base_stateful {
    ($name:ident) => {
        paste::paste! {
            impl [<Stateful $name>] {
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
        fn is_over(&self) -> Result<bool, error::Error> {
            self.is_over()
        }

        #[inline(always)]
        fn type_str(&self) -> String {
            self.type_str()
        }
    };
}
