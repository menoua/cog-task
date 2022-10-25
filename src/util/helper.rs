use serde::Serialize;
use spin_sleep::{SpinSleeper, SpinStrategy};

const SPIN_DURATION: u32 = 100_000_000; // equivalent to 100ms
const SPIN_STRATEGY: SpinStrategy = SpinStrategy::SpinLoopHint;

#[inline(always)]
pub fn spin_sleeper() -> SpinSleeper {
    SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY)
}

#[inline(always)]
pub fn f32_with_precision(x: f32, precision: u8) -> f32 {
    let precision = 10_f32.powi(precision as i32);
    (x * precision).round() / precision
}

#[inline(always)]
pub fn f64_with_precision(x: f32, precision: u8) -> f64 {
    let shift = 10_f64.powi(precision as i32);
    (x as f64 * shift).round() / shift
}

#[inline]
pub fn str_with_precision(x: f32, precision: u8) -> String {
    let shift = 10_f64.powi(precision as i32);
    let string = format!("{}", (x as f64 * shift).round());
    let (int, frac) = string.split_at(string.len() - precision as usize);
    let int = if int.is_empty() { "0" } else { int };
    if frac.is_empty() {
        int.to_owned()
    } else {
        format!("{int}.{frac}")
    }
}

pub trait Hash: Serialize {
    fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::default();
        hasher.update(&serde_cbor::to_vec(&self).unwrap());
        hex::encode(hasher.finalize())
    }
}
