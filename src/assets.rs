use spin_sleep::SpinStrategy;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const TEXT_TITLE: u16 = 45;
pub const TEXT_XLARGE: u16 = 40;
pub const TEXT_LARGE: u16 = 36;
pub const TEXT_NORMAL: u16 = 34;
pub const TEXT_SMALL: u16 = 32;
pub const TEXT_XSMALL: u16 = 28;
pub const TEXT_TINY: u16 = 24;

pub const SPIN_DURATION: u32 = 100_000_000; // equivalent to 100ms
pub const SPIN_STRATEGY: SpinStrategy = SpinStrategy::SpinLoopHint;

pub const IMAGE_FIXATION: &[u8] = include_bytes!("assets/fixation.svg");
pub const IMAGE_RUSTACEAN: &[u8] = include_bytes!("assets/rustacean.svg");

pub const ICON_HELP: &[u8] = include_bytes!("assets/help-2.svg");
pub const ICON_SYSTEM_INFO: &[u8] = include_bytes!("assets/system-info-3.svg");
pub const ICON_TO_CLIPBOARD: &[u8] = include_bytes!("assets/to-clipboard.svg");
pub const ICON_CLOSE_WINDOW: &[u8] = include_bytes!("assets/close-window-1.svg");
pub const ICON_SINGLE_FOLDER: &[u8] = include_bytes!("assets/folder-1.svg");
pub const ICON_MULTI_FOLDERS: &[u8] = include_bytes!("assets/folders.svg");
pub const ICON_MAGNIFYING_GLASS: &[u8] = include_bytes!("assets/magnifying-glass.svg");
