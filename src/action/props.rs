pub const DEFAULT: u64 = 0;
pub const FINITE: u64 = 1 << 1;
pub const VISUAL: u64 = 1 << 2;
pub const ANIMATED: u64 = 2 << 3;
pub const CAP_KEYS: u64 = 3 << 4;

pub struct Props(u64);

impl Props {
    pub fn new(bitmask: u64) -> Self {
        Self(bitmask)
    }

    pub fn finite(&self) -> bool {
        (self.0 & FINITE) != 0
    }

    pub fn visual(&self) -> bool {
        (self.0 & VISUAL) != 0
    }

    pub fn animated(&self) -> bool {
        (self.0 & ANIMATED) != 0
    }

    pub fn captures_keys(&self) -> bool {
        (self.0 & CAP_KEYS) != 0
    }
}

impl From<u64> for Props {
    fn from(bitmask: u64) -> Self {
        Self(bitmask)
    }
}
