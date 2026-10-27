//! A small SVG path reader, for the hogwarts sprites drawn after the home
//! page's own SVG (banners and crests, the owl, the seal, the hourglass).
//! It knows M L H V C Q Z, absolute and relative, with implicit repeats, and
//! two shorthands of ours: `O cx cy r` (a circle) and `E cx cy rx ry deg`
//! (a rotated ellipse).

// drawing code: x, y, w, h for a box, c for the canvas, as in the SVG it follows
#![allow(clippy::many_single_char_names)]

use femtovg::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Command(char),
    Number(f32),
}

fn tokens(d: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let chars: Vec<char> = d.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() {
            out.push(Token::Command(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '.' || c == '+' {
            // a number: sign, digits, one dot; a second dot starts the next one
            let start = i;
            if c == '-' || c == '+' {
                i += 1;
            }
            let mut dot = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || (chars[i] == '.' && !dot)) {
                dot |= chars[i] == '.';
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(Token::Number(text.parse().unwrap_or(0.0)));
        } else {
            i += 1;
        }
    }
    out
}

/// The femtovg path of an SVG path string.
pub fn path(d: &str) -> Path {
    let mut pen = Path::new();
    trace(d, &mut pen);
    pen
}

fn trace(d: &str, pen: &mut Path) {
    let tokens = tokens(d);
    let mut i = 0;
    let (mut x, mut y, mut start_x, mut start_y) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    let mut command = 'M';
    let numbers = |i: &mut usize, count: usize| -> Option<Vec<f32>> {
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            match tokens.get(*i) {
                Some(Token::Number(value)) => {
                    out.push(*value);
                    *i += 1;
                }
                _ => return None,
            }
        }
        Some(out)
    };
    while i < tokens.len() {
        if let Token::Command(c) = tokens[i] {
            command = c;
            i += 1;
            if c == 'Z' || c == 'z' {
                pen.close();
                (x, y) = (start_x, start_y);
            }
            continue;
        }
        let relative = command.is_ascii_lowercase();
        let (ox, oy) = if relative { (x, y) } else { (0.0, 0.0) };
        match command.to_ascii_uppercase() {
            'M' => {
                let Some(v) = numbers(&mut i, 2) else { break };
                (x, y) = (ox + v[0], oy + v[1]);
                (start_x, start_y) = (x, y);
                pen.move_to(x, y);
                // the pairs after a move are lines
                command = if relative { 'l' } else { 'L' };
            }
            'L' => {
                let Some(v) = numbers(&mut i, 2) else { break };
                (x, y) = (ox + v[0], oy + v[1]);
                pen.line_to(x, y);
            }
            'H' => {
                let Some(v) = numbers(&mut i, 1) else { break };
                x = ox + v[0];
                pen.line_to(x, y);
            }
            'V' => {
                let Some(v) = numbers(&mut i, 1) else { break };
                y = oy + v[0];
                pen.line_to(x, y);
            }
            'C' => {
                let Some(v) = numbers(&mut i, 6) else { break };
                let c = [
                    ox + v[0],
                    oy + v[1],
                    ox + v[2],
                    oy + v[3],
                    ox + v[4],
                    oy + v[5],
                ];
                pen.bezier_to(c[0], c[1], c[2], c[3], c[4], c[5]);
                (x, y) = (c[4], c[5]);
            }
            'Q' => {
                let Some(v) = numbers(&mut i, 4) else { break };
                let c = [ox + v[0], oy + v[1], ox + v[2], oy + v[3]];
                pen.quad_to(c[0], c[1], c[2], c[3]);
                (x, y) = (c[2], c[3]);
            }
            'O' => {
                let Some(v) = numbers(&mut i, 3) else { break };
                pen.circle(v[0], v[1], v[2]);
            }
            'E' => {
                let Some(v) = numbers(&mut i, 5) else { break };
                let (cx, cy, rx, ry, angle) = (v[0], v[1], v[2], v[3], v[4].to_radians());
                for step in 0..=24 {
                    let a = step as f32 / 24.0 * std::f32::consts::TAU;
                    let (ex, ey) = (rx * a.cos(), ry * a.sin());
                    let (px, py) = (
                        cx + ex * angle.cos() - ey * angle.sin(),
                        cy + ex * angle.sin() + ey * angle.cos(),
                    );
                    if step == 0 {
                        pen.move_to(px, py);
                    } else {
                        pen.line_to(px, py);
                    }
                }
                pen.close();
            }
            _ => i += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use femtovg::Verb;

    /// The end point of every segment of `d`.
    fn points(d: &str) -> Vec<(f32, f32)> {
        path(d)
            .verbs()
            .filter_map(|verb| match verb {
                Verb::MoveTo(x, y) | Verb::LineTo(x, y) | Verb::BezierTo(_, _, _, _, x, y) => {
                    Some((x, y))
                }
                _ => None,
            })
            .map(|(x, y)| ((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0))
            .collect()
    }

    #[test]
    fn numbers_run_together_as_svg_writes_them() {
        assert_eq!(
            tokens("M1.6.4-2L3"),
            vec![
                Token::Command('M'),
                Token::Number(1.6),
                Token::Number(0.4),
                Token::Number(-2.0),
                Token::Command('L'),
                Token::Number(3.0),
            ]
        );
    }

    #[test]
    fn pairs_after_a_move_are_lines_and_curves_repeat() {
        assert_eq!(
            points("M0 0 10 0 10 10Z"),
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        );
        assert_eq!(
            points("M0 0C1 1 2 2 3 3 4 4 5 5 6 6"),
            vec![(0.0, 0.0), (3.0, 3.0), (6.0, 6.0)]
        );
    }

    #[test]
    fn relative_commands_move_from_the_pen() {
        assert_eq!(
            points("M45 40l1.6 1.6h2v-1"),
            vec![(45.0, 40.0), (46.6, 41.6), (48.6, 41.6), (48.6, 40.6)]
        );
    }
}
