#![feature(let_chains)]

use std::time::Instant;

use macroquad::prelude::*;
use num::{Complex, ToPrimitive};
use rust_lab::polynomials::{root_locus::RootLocus, roots::PolynomialRoot, Polynomial};

enum Mode {
    Zoom,
    Interval,
    Precision,
}

#[macroquad::main("Root Locus")]
async fn main() {
    const COLORS: [Color; 12] = [
        RED, GREEN, BLUE, YELLOW, PINK, BROWN, BEIGE, LIME, LIGHTGRAY, PURPLE, ORANGE, MAGENTA,
    ];

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
    let mut dragging_root: Option<usize> = None;
    let mut dragging_plot: Option<(f32, f32)> = None;

    let mut sx = 292.91;
    let mut sy = -205.19;

    let mut ox = 763.63;
    let mut oy = 300.0;

    let mut mode = Mode::Zoom;
    let mut interval = 1.01;
    let mut precision = 1e-6;

    loop {
        i += 1;
        let t1 = Instant::now();

        let poly_a = Polynomial::from_roots(1.0f32, &a_roots);
        let poly_b = Polynomial::from_roots(1.0, &b_roots);

        // println!("A(x) = {}\nB(x) = {}", poly_a, poly_b);

        let mut rl = RootLocus::new(poly_a.clone(), poly_b.clone());

        rl.calculate_all(precision, interval, 0.01, 1000.0);

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

        if is_key_pressed(KeyCode::M) {
            mode = match mode {
                Mode::Zoom => Mode::Interval,
                Mode::Interval => Mode::Precision,
                Mode::Precision => Mode::Zoom,
            }
        }

        // Scale calculation

        if is_key_down(KeyCode::R) {
            let width = screen_width();
            let height = screen_height();

            let dx = (max_re - min_re) * 1.1;
            let dy = (max_im - min_im) * 1.1;

            sx = width / dx;
            sy = -height / dy;

            ox = width / 2.0 - sx * (min_re + max_re) * 0.5;
            oy = height / 2.0 + sy * (min_im + max_im) * 0.5;
        } else {
            let wheel = (mouse_wheel().1 / 120.0).to_i32().unwrap();

            match mode {
                Mode::Zoom => {
                    let (mx, my) = mouse_position();

                    if wheel != 0 {
                        let val = 1.05_f32.powi(wheel);
                        sx *= val;
                        sy *= val;
                        // Have to adjust offsets to scale around the mouse
                    } else if let Some((ix, iy)) = dragging_plot {
                        if !is_mouse_button_down(MouseButton::Middle) {
                            dragging_plot = None;
                        } else {
                            ox += mx - ix;
                            oy += my - iy;
                            dragging_plot = Some((mx, my));
                        }
                    } else if is_mouse_button_down(MouseButton::Middle) {
                        dragging_plot = Some((mx, my));
                    }
                }
                Mode::Interval => {
                    if wheel != 0 {
                        let val = 1.001_f32.powi(wheel);
                        interval *= val;
                        if interval <= 1.0 {
                            interval = 1.005
                        }
                    }
                }
                Mode::Precision => {
                    if wheel != 0 {
                        let val = 10_f32.powi(wheel);
                        precision *= val;
                    }
                }
            }
        }

        if is_mouse_button_down(MouseButton::Left) {
            // First, convert from screen to plot axis
            let mut mx = (mouse_position().0 - ox) / sx;
            let mut my = (mouse_position().1 - oy) / sy;

            if mx.abs() < 5.0 / sx {
                mx = 0.0;
            }
            #[allow(unused_assignments)]
            if my.abs() < 5.0 / sx {
                my = 0.0;
            }

            let comp_mouse = Complex::new(mx, my);

            if let Some(r) = dragging_root {
                let root = if r < a_roots.len() {
                    &mut a_roots[r]
                } else {
                    &mut b_roots[r - a_roots.len()]
                };
                if my == 0.0 {
                    match root {
                        &mut PolynomialRoot::RealSingle(ref mut rx) => *rx = mx,
                        &mut PolynomialRoot::ComplexPair(_) => {
                            *root = PolynomialRoot::RealSingle(mx)
                        }
                    }
                } else {
                    match root {
                        &mut PolynomialRoot::RealSingle(_) => {
                            *root = PolynomialRoot::ComplexPair(comp_mouse)
                        }
                        &mut PolynomialRoot::ComplexPair(ref mut c) => {
                            c.re = mx;
                            c.im = my;
                        }
                    }
                }
            } else {
                for (i, root) in a_roots.iter_mut().chain(b_roots.iter_mut()).enumerate() {
                    // Check distance
                    let dist_sqr = (root.as_complex() - comp_mouse).norm_sqr();

                    if dist_sqr > 100.0 / (sx * sx) {
                        continue;
                    }

                    if my == 0.0 {
                        match root {
                            &mut PolynomialRoot::RealSingle(ref mut rx) => *rx = mx,
                            &mut PolynomialRoot::ComplexPair(_) => {
                                *root = PolynomialRoot::RealSingle(mx)
                            }
                        }
                    } else {
                        match root {
                            &mut PolynomialRoot::RealSingle(_) => {
                                *root = PolynomialRoot::ComplexPair(comp_mouse)
                            }
                            &mut PolynomialRoot::ComplexPair(ref mut c) => {
                                c.re = mx;
                                c.im = my;
                            }
                        }
                    }

                    dragging_root = Some(i);
                    break;
                }
            }
        } else {
            dragging_root = None;
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

        let radius = 2.0;

        for p in a_roots.iter() {
            match p {
                PolynomialRoot::RealSingle(r) => {
                    draw_circle(r * sx + ox, oy, radius, WHITE);
                }
                PolynomialRoot::ComplexPair(c) => {
                    draw_circle(c.re * sx + ox, c.im * sy + oy, radius, WHITE);
                    draw_circle(c.re * sx + ox, -c.im * sy + oy, radius, WHITE);
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
                    draw_rectangle(
                        c.re * sx + ox - radius,
                        -c.im * sy + oy - radius,
                        2.0 * radius,
                        2.0 * radius,
                        WHITE,
                    );
                }
            }
        }

        let dur = t1.elapsed();
        tot += dur.as_nanos();

        match mode {
            Mode::Interval => {
                draw_text(
                    &format!("Interval: {:.4}", interval),
                    5.0,
                    screen_height() - 15.0,
                    30.0,
                    WHITE,
                );
            }
            Mode::Precision => {
                draw_text(
                    &format!("Precision: {}", precision),
                    5.0,
                    screen_height() - 15.0,
                    30.0,
                    WHITE,
                );
            }
            _ => {}
        }

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
