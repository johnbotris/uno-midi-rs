use crate::maths;

/// Determine whether pulse is on or off for a given euclidean rhythm
pub fn euclidean(steps: u64, pulses: u64, rotation: i64, step: u64) -> bool {
    if steps == 0 || pulses == 0 {
        return false;
    }

    if pulses == steps {
        return true;
    }

    let target_step = maths::modulo(step as i64 - rotation, steps);
    let mut bucket = 0;
    let mut step_value = false;
    for step in 0..=target_step {
        step_value = false;
        bucket += pulses;
        if (bucket >= steps) {
            bucket -= steps;
            step_value = true
        }
    }
    return step_value;
}
