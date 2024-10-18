#![feature(let_chains)]
mod rng;
mod plot;

use clap::{Parser, Subcommand};


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

async fn macroquad_main() {
    plot::macroquad::mainloop().await;
}

fn main() {
    let args = Args::parse();

    match args.frontend {
        FrontEnd::WGPU => pollster::block_on(plot::wgpu::run()),
        FrontEnd::Macroquad => pollster::block_on(plot::macroquad::mainloop()),
    }
}