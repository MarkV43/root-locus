use std::time::Instant;

use macroquad::prelude::*;
use rust_lab::polynomials::{root_locus::RootLocus, Polynomial};

#[macroquad::main("BasicShapes")]
async fn main() {
    const COLORS: [Color; 4] = [RED, GREEN, BLUE, YELLOW];

    let mut i = 0;

    let mut tot = 0;

    loop {
        i += 1;
        let t1 = Instant::now();

        clear_background(BLACK);

        let poly_a = Polynomial::from_roots(1.0f32, &[0.0, -1.0]);
        let poly_b = Polynomial::from_roots(1.0, &[-2.0, -3.0]);
        let mut rl = RootLocus::new(poly_a.clone(), poly_b.clone());

        rl.calculate_all(1e-12, 1.05, 0.01, 100.0);

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

        let dur = t1.elapsed();
        tot += dur.as_nanos();

        if i % 60 == 0 {
            println!("{} %", tot as f32 / 1_000_000_000_f32 * 100.0);
            i = 0;
            tot = 0;
        }

        next_frame().await
    }
}
