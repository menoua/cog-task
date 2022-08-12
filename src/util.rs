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

#[inline(always)]
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
