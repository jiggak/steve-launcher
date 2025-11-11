// This was ported from:
// https://github.com/meza/curseforge-fingerprint/blob/main/src/addon/fingerprint.cpp
// ... and therefore the original license applies

pub fn curseforge_hash(buffer: &Vec<u8>) -> u32 {
    const MULTIPLEX: u32 = 1540483477;
    const SEED: u32 = 1;

    let num1 = compute_normalized_length(buffer) as u32;
    let mut num2 = SEED ^ num1;
    let mut num3: u32 = 0;
    let mut num4: u32 = 0;

    for b in buffer.iter().filter(|b| !is_whitespace_character(**b)) {
        num3 |= (*b as u32) << num4;
        num4 += 8;
        if num4 == 32 {
            // let num6 = num3 * MULTIPLEX;
            let num6 = num3.wrapping_mul(MULTIPLEX);

            // let num7 = (num6 ^ num6 >> 24) * MULTIPLEX;
            let num7 = (num6 ^ num6 >> 24).wrapping_mul(MULTIPLEX);

            // num2 = num2 * MULTIPLEX ^ num7;
            num2 = num2.wrapping_mul(MULTIPLEX) ^ num7;
            num3 = 0;
            num4 = 0;
        }
    }

    if num4 > 0 {
        // num2 = (num2 ^ num3) * MULTIPLEX;
        num2 = (num2 ^ num3).wrapping_mul(MULTIPLEX);
    }

    // let num6 = (num2 ^ num2 >> 13) * MULTIPLEX;
    let num6 = (num2 ^ num2 >> 13).wrapping_mul(MULTIPLEX);

    return num6 ^ num6 >> 15;
}

fn compute_normalized_length(buffer: &Vec<u8>) -> usize {
    buffer.iter()
        .filter(|b| !is_whitespace_character(**b))
        .count()
}

fn is_whitespace_character(b: u8) -> bool {
    b == 9 || b == 10 || b == 13 || b == 32
}
