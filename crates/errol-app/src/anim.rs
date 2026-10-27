//! Easing, the decoding text effect, and a deterministic hash for everything
//! that must look random but stay put from one frame to the next.

/// 0 → 1, slowing down at the end.
pub fn ease_out(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    1.0 - (1.0 - k).powi(3)
}

/// 0 → 1, overshooting a little before settling (entrances).
pub fn ease_back(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    let c = 1.6_f32;
    1.0 + (c + 1.0) * (k - 1.0).powi(3) + c * (k - 1.0).powi(2)
}

pub fn ease_in_out(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Progress of an animation of `length` seconds started `elapsed` seconds ago.
pub fn progress(elapsed: f32, length: f32) -> f32 {
    if length <= 0.0 {
        1.0
    } else {
        (elapsed / length).clamp(0.0, 1.0)
    }
}

/// A well-mixed 32-bit hash of a few integers (splitmix-like), to draw
/// "random" glyphs and positions that do not jump between frames.
pub fn hash(a: u64, b: u64) -> u64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0x6A09_E667_F3BC_C909);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A float in 0..1 from a hash.
pub fn unit(a: u64, b: u64) -> f32 {
    (hash(a, b) >> 40) as f32 / (1u64 << 24) as f32
}

/// `text` resolving from random glyphs, left to right, over `length`
/// seconds; the glyphs change 25 times a second.
pub fn decode(text: &str, elapsed: f32, length: f32, glyphs: &str) -> String {
    let done = progress(elapsed, length);
    if done >= 1.0 {
        return text.to_owned();
    }
    let pool: Vec<char> = glyphs.chars().collect();
    let count = text.chars().count();
    let shown = (count as f32 * done) as usize;
    let tick = (elapsed * 25.0) as u64;
    text.chars()
        .enumerate()
        .map(|(index, c)| {
            if index < shown || c == ' ' {
                c
            } else {
                pool[(hash(tick, index as u64) % pool.len() as u64) as usize]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decoded_text_ends_as_itself_and_keeps_its_spaces_meanwhile() {
        assert_eq!(decode("Néon allumé", 2.0, 1.0, "01"), "Néon allumé");
        assert_eq!(decode("ab cd", 0.0, 1.0, "#").chars().nth(2), Some(' '));
    }

    #[test]
    fn eases_start_at_zero_and_end_at_one() {
        for ease in [ease_out, ease_back, ease_in_out] {
            assert!(ease(0.0).abs() < 1e-5 && (ease(1.0) - 1.0).abs() < 1e-5);
        }
    }
}
