//! Adobe standard glyph widths for the two base-14 fonts the receipt uses.
//!
//! Widths are in 1/1000 em, so a string's width at size `s` is
//! `sum(width) * s / 1000`. Without these the renderer cannot right-align a
//! column, centre a line, or size a badge to its label -- it can only guess
//! from character counts, which drifts badly for digits vs. narrow letters
//! ("Mode: PhonePe" and "Date: 09/08/2026" differ by 30pt at the same length).

/// Helvetica, codepoints 32..=126.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // 32..47
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 48..57 digits
    278, 278, 584, 584, 584, 556, 1015, // 58..64
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667,
    611, 722, 667, 944, 667, 667, 611, // 65..90
    278, 278, 278, 469, 556, 333, // 91..96
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500,
    278, 556, 500, 722, 500, 500, 500, // 97..122
    334, 260, 334, 584, // 123..126
];

/// Helvetica-Bold, codepoints 32..=126.
const HELVETICA_BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, // 32..47
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 48..57 digits
    333, 333, 584, 584, 584, 611, 975, // 58..64
    722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667,
    611, 722, 667, 944, 667, 667, 611, // 65..90
    333, 278, 333, 584, 556, 333, // 91..96
    556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611, 611, 611, 389, 556,
    333, 611, 556, 778, 556, 556, 500, // 97..122
    389, 280, 389, 584, // 123..126
];

/// Width of one already-sanitised character, in 1/1000 em.
fn glyph_width(c: char, bold: bool) -> u16 {
    let table = if bold { &HELVETICA_BOLD } else { &HELVETICA };
    match c {
        ' '..='~' => table[c as usize - 32],
        // The handful of WinAnsi extras `sanitize` is allowed to emit.
        '\u{b7}' => 333,                // middle dot
        '\u{2013}' | '\u{2022}' => 556, // en dash, bullet
        '\u{2014}' => 1000,             // em dash
        '\u{2018}' | '\u{2019}' => {
            if bold {
                238
            } else {
                191
            }
        }
        '\u{201c}' | '\u{201d}' => {
            if bold {
                474
            } else {
                333
            }
        }
        // Anything else never reaches the page; treat as a space so a stray
        // char cannot silently shrink a measured column.
        _ => table[0],
    }
}

/// Width of a sanitised string at `size` points.
///
/// Must be given the SAME text that will be drawn (post-`sanitize`), otherwise
/// alignment computed here will not match what the viewer renders.
pub fn text_width(s: &str, size: f32, bold: bool) -> f32 {
    s.chars().map(|c| glyph_width(c, bold) as f32).sum::<f32>() * size / 1000.0
}
