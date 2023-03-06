use eframe::egui;
use serde::{Deserialize, Serialize};

macro_rules! key {
    ($($name:ident),* $(,)?) => {
        #[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[serde(rename_all = "snake_case")]
        pub enum Key {
            $($name,)*
        }

        impl From<&Key> for egui::Key {
            #[inline]
            fn from(k: &Key) -> Self {
                match k {
                    $(
                        Key::$name => egui::Key::$name,
                    )*
                }
            }
        }

        impl From<Key> for egui::Key {
            #[inline(always)]
            fn from(k: Key) -> Self {
                Self::from(&k)
            }
        }

        impl From<&egui::Key> for Key {
            #[inline]
            fn from(k: &egui::Key) -> Self {
                match k {
                    $(
                        egui::Key::$name => Key::$name,
                    )*
                }
            }
        }

        impl From<egui::Key> for Key {
            #[inline(always)]
            fn from(k: egui::Key) -> Self {
                Self::from(&k)
            }
        }
    }
}

key!(
    ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Escape, Tab, Backspace, Enter, Space, Insert,
    Delete, Home, End, PageUp, PageDown, Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8,
    Num9, Minus, PlusEquals, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W,
    X, Y, Z, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19,
    F20,
);
