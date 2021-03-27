pub const fn modulo(dividend: i64, divisor: u64) -> u64 {
    let divisor: i64 = divisor as i64;
    ((dividend % divisor + divisor) % divisor) as u64
}
