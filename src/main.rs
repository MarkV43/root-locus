use std::time::Instant;

use macroquad::prelude::*;
use num::Float;
use rust_lab::polynomials::{root_locus::RootLocus, roots::PolynomialRoot, Polynomial};

#[macroquad::main("Root Locus")]
async fn main() {
    const COLORS: [Color; 4] = [RED, GREEN, BLUE, YELLOW];

    let mut i = 0;

    let mut tot = 0;

    let mut a_roots = vec![
        PolynomialRoot::RealSingle(0.0),
        PolynomialRoot::RealSingle(0.0),
        PolynomialRoot::RealSingle(-0.5),
    ];
    let mut b_roots = vec![
        PolynomialRoot::RealSingle(-1.0),
        PolynomialRoot::RealSingle(-2.0),
        PolynomialRoot::RealSingle(-2.5),
    ];

    let mut filter = None;
    let mut dragging: Option<usize> = None;

    loop {
        i += 1;
        let t1 = Instant::now();

        let poly_a = Polynomial::from_roots(1.0f32, &a_roots);
        let poly_b = Polynomial::from_roots(1.0, &b_roots);

        // println!("A(x) = {}\nB(x) = {}", poly_a, poly_b);

        let mut rl = RootLocus::new(poly_a.clone(), poly_b.clone());

        rl.calculate_all(1e-12, 1.01, 0.01, 1000.0);

        let all_roots = rl.get_roots();

        let mut min_re = f32::INFINITY;
        let mut min_im = f32::INFINITY;
        let mut max_re = -f32::INFINITY;
        let mut max_im = -f32::INFINITY;

        for x in all_roots.iter() {
            if x.re < min_re {
                min_re = x.re;
            }
            if x.re > max_re {
                max_re = x.re;
            }
            if x.im < min_im {
                min_im = x.im;
            }
            if x.im > max_im {
                max_im = x.im;
            }
        }

        let width = screen_width();
        let height = screen_height();

        let dx = (max_re - min_re) * 1.1;
        let dy = (max_im - min_im) * 1.1;

        let sx = width / dx;
        let sy = -height / dy;

        let ox = width / 2.0 - sx * (min_re + max_re) * 0.5;
        let oy = height / 2.0 + sy * (min_im + max_im) * 0.5;

        if is_mouse_button_down(MouseButton::Left) {
            // First, convert from screen to plot axis
            let mut mx = (mouse_position().0 - ox) / sx;
            let mut my = (mouse_position().1 - oy) / sy;

            if mx.abs() < 5.0 / sx {
                mx = 0.0;
            }
            if my.abs() < 5.0 / sx {
                my = 0.0;
            }

            if let Some(r) = dragging {
                if r < a_roots.len() {
                    if let PolynomialRoot::RealSingle(ref mut a) = a_roots[r] {
                        *a = mx;
                    }
                } else {
                    if let PolynomialRoot::RealSingle(ref mut b) = b_roots[r - a_roots.len()] {
                        *b = mx;
                    }
                }
            } else {
                for (i, root) in a_roots.iter_mut().chain(b_roots.iter_mut()).enumerate() {
                    if let &mut PolynomialRoot::RealSingle(ref mut root) = root {
                        if (*root - mx).abs() < 5.0 / sx {
                            dragging = Some(i);
                            *root = mx;
                            break;
                        }
                    }
                }
            }
        } else {
            dragging = None;
        }

        let last_key = get_last_key_pressed();

        if let Some(code) = last_key {
            filter = match code {
                KeyCode::Key1 => Some(0),
                KeyCode::Key2 => Some(1),
                KeyCode::Key3 => Some(2),
                KeyCode::Key4 => Some(3),
                KeyCode::Key5 => Some(4),
                KeyCode::Key0 => None,
                _ => filter,
            }
        }

        clear_background(BLACK);

        if let Some(branch) = filter {
            for roots in all_roots
                .iter()
                .skip(branch)
                .step_by(rl.get_branches())
                .collect::<Vec<_>>()
                .windows(2)
            {
                let p = roots[1];
                let q = roots[0];
                draw_line(
                    p.re * sx + ox,
                    p.im * sy + oy,
                    q.re * sx + ox,
                    q.im * sy + oy,
                    2.0,
                    COLORS[branch],
                );
            }
        } else {
            for roots in all_roots
                .chunks(rl.get_branches())
                .collect::<Vec<_>>()
                .windows(2)
            {
                for (i, (p, q)) in roots[0].iter().zip(roots[1].iter()).enumerate() {
                    draw_line(
                        p.re * sx + ox,
                        p.im * sy + oy,
                        q.re * sx + ox,
                        q.im * sy + oy,
                        2.0,
                        COLORS[i],
                    );
                }
            }
        }

        //   CG
        // ------
        // 1 + CG
        //
        // 1 + CG = 0
        // C = K
        // Gd + K Gn = 0
        // A + K B = 0
        let radius = 2.0;

        for p in a_roots.iter() {
            match p {
                PolynomialRoot::RealSingle(r) => {
                    draw_circle(r * sx + ox, oy, radius, WHITE);
                }
                PolynomialRoot::ComplexPair(c) => {
                    draw_circle(c.re * sx + ox, c.im * sy + oy, radius, WHITE);
                }
            }
        }

        for z in b_roots.iter() {
            match z {
                PolynomialRoot::RealSingle(r) => {
                    draw_rectangle(
                        r * sx + ox - radius,
                        oy - radius,
                        2.0 * radius,
                        2.0 * radius,
                        WHITE,
                    );
                }
                PolynomialRoot::ComplexPair(c) => {
                    draw_rectangle(
                        c.re * sx + ox - radius,
                        c.im * sy + oy - radius,
                        2.0 * radius,
                        2.0 * radius,
                        WHITE,
                    );
                }
            }
        }

        let dur = t1.elapsed();
        tot += dur.as_nanos();

        if i % 60 == 0 {
            println!(
                "{:.4} % \t for {} points",
                tot as f32 / 1_000_000_000_f32 * 100.0,
                all_roots.len() / rl.get_branches()
            );
            i = 0;
            tot = 0;
        }

        next_frame().await
    }
}
