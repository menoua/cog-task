macro_rules! include_actions {
    ($($group:ident::$name:ident@($($feature:literal),* $(,)?)),* $(,)?) => {
        use crate::action::Action;
        use eyre::Result;
        use serde::{Deserialize, Serialize};
        use std::any::TypeId;

        paste::paste! {
            $(
                #[cfg(all($(feature = $feature,)*))]
                pub use crate::action::$group::$name;
            )*
        }

        paste::paste! {
            $(
                #[cfg(all($(feature = $feature,)*))]
                pub use crate::action::$group::$name::[<$name:camel>];
            )*
        }

        paste::paste! {
            #[derive(Deserialize, Serialize)]
            #[serde(rename_all = "snake_case")]
            pub enum ActionEnum {
                $(
                    #[cfg(all($(feature = $feature,)*))]
                    [<$name:camel>]([<$name:camel>]),
                )*
            }

            impl ActionEnum {
                pub fn unwrap(self) -> Result<Box<dyn Action>> {
                    match self {
                        $(
                            #[cfg(all($(feature = $feature,)*))]
                            Self::[<$name:camel>](inner) => inner.init(),
                        )*
                    }
                }

                pub fn as_ref(&self) -> ActionEnumAsRef {
                    match self {
                        $(
                            #[cfg(all($(feature = $feature,)*))]
                            Self::[<$name:camel>](inner) => ActionEnumAsRef::[<$name:camel>](inner),
                        )*
                    }
                }
            }

            #[derive(Serialize)]
            #[serde(rename_all = "snake_case")]
            pub enum ActionEnumAsRef<'a> {
                $(
                    #[cfg(all($(feature = $feature,)*))]
                    [<$name:camel>](&'a [<$name:camel>]),
                )*
            }

            impl<'a> From<&'a dyn Action> for ActionEnumAsRef<'a> {
                fn from(f: &dyn Action) -> ActionEnumAsRef {
                    match f.type_id() {
                        $(
                            #[cfg(all($(feature = $feature,)*))]
                            t if t == TypeId::of::<[<$name:camel>]>() => unsafe {
                                ActionEnumAsRef::[<$name:camel>](
                                    &*(f as *const dyn Action as *const [<$name:camel>])
                                )
                            }
                        )*
                        t => panic!("Unknown action type ({t:?})"),
                    }
                }
            }
        }
    }
}

macro_rules! include_stateful_actions {
    ($($group:ident::$name:ident@($($feature:literal),* $(,)?)),* $(,)?) => {
        paste::paste!(
            $(
                #[cfg(all($(feature = $feature,)*))]
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
                fn is_over(&self) -> Result<bool> {
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
                done: Arc<Mutex<Result<bool>>>,
                $(
                    $field: $ty,
                )*
            }

            impl_base_stateful!($name);

            impl [<Stateful $name>] {
                #[inline(always)]
                fn is_over(&self) -> Result<bool> {
                    use eyre::{eyre, Context};

                    let mut done = self.done.lock().unwrap();
                    match &*done {
                        Ok(c) => Ok(*c),
                        Err(_) => {
                            let e = std::mem::replace(&mut *done, Err(eyre!("")));
                            e.wrap_err("Detected failure while checking `is_over`.")
                        }
                    }
                }
            }
        }
    }
}

macro_rules! impl_stateful {
    () => {
        #[inline(always)]
        fn is_over(&self) -> Result<bool> {
            self.is_over()
        }

        #[inline(always)]
        fn type_str(&self) -> String {
            self.type_str()
        }
    };
}
