use std::time::Instant;

use macroquad::prelude::*;
use rust_lab::{
    polynomials::{root_locus::RootLocus, Polynomial},
    NotNanFloat,
};

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

        // Recalculate these

        let min_re = all_roots
            .iter()
            .map(|x| NotNanFloat::new(x.re))
            .min()
            .unwrap()
            .0;
        let min_im = all_roots
            .iter()
            .map(|x| NotNanFloat::new(x.im))
            .min()
            .unwrap()
            .0;
        let max_re = all_roots
            .iter()
            .map(|x| NotNanFloat::new(x.re))
            .max()
            .unwrap()
            .0;
        let max_im = all_roots
            .iter()
            .map(|x| NotNanFloat::new(x.im))
            .max()
            .unwrap()
            .0;

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
