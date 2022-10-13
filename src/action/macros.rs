macro_rules! include_actions {
    ($($name:ident),* $(,)?) => {
        use crate::action::Action;

        $(
            pub mod $name;
        )*

        paste::paste! {
            $(
                pub use crate::action::[<$name>]::[<$name:camel>];
            )*
        }

        pub fn from_name_and_fields(
            name: &str,
            fields: Vec<u8>
        ) -> Result<Option<Box<dyn Action>>, serde_json::Error> {
            use serde::Deserialize;

            fn boxed<'de, T: 'static + Action + Deserialize<'de>>(
                fields: &'de [u8],
            ) -> Result<Box<dyn Action>, serde_json::Error> {
                let action: T = serde_json::from_slice(fields)?;
                Ok(Box::new(action))
            }

            let action = match name {
                $(
                    paste::paste!(stringify!([<$name:snake>])) => {
                        paste::paste! {
                            Some(boxed::<[<$name:camel>]>(&fields))
                        }
                    }
                )*
                _ => None,
            };

            match action {
                Some(Ok(action)) => Ok(Some(action)),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            }
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
                    String::from(stringify!($name:snake))
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
