#![feature(let_chains)]
mod plot;
mod rng;

use clap::{Parser, Subcommand};
use num::{Signed, Zero};

#[derive(Subcommand, Debug)]
enum FrontEnd {
    Macroquad,
    WGPU,
}

/// Binary that draws a dynamic root-locus plot
#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// Type of frontend to be used
    #[command(subcommand)]
    frontend: FrontEnd,
}

fn main() {
    /* use rust_lab::polynomials::roots::RootFinding;
    use std::f64::consts::PI;

    let M = 1.0;
    let h = 0.6;

    let pol = rust_lab::polynomials::Polynomial64::new(vec![
        -2000.0 * M,
        100.0 * PI * h,
        200.0 * M,
        101.0 * PI * h,
        0.0,
        PI * h,
    ]);
    let mut roots = vec![num::complex::Complex64::zero(); pol.order()];
    pol.find_roots(&mut roots, 1e-12);

    for root in roots.into_iter() {
        // if root.re.is_sign_positive() {
        println!("{root}");
        // }
    }

    return; */
    let args = Args::parse();

    match args.frontend {
        FrontEnd::WGPU => pollster::block_on(plot::wgpu::run()),
        FrontEnd::Macroquad => macroquad::Window::new("Root Locus", plot::macroquad::mainloop()),
    }
}
