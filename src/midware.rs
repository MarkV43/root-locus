use std::fmt::{Debug, Display};

use num::Float;
use rand::{distributions::Standard, prelude::Distribution};

use crate::rng::generate_rng;
use crate::polynomials::root_locus::RootLocus;

/// Describes the camera's position and zoom
pub struct View {
    scale: [f32; 2],
    center: [f32; 2],
}

impl Default for View {
    fn default() -> Self {
        Self {
            scale: [1.0, 1.0],
            center: [-0.5, 0.0],
        }
    }
}

/// Stores information about the camera view,
/// algorithm precision, gain invervals,
/// and transfer function being plotted.
pub struct Midware<F: Float> {
    poles: Vec<F>,
    zeros: Vec<F>,
    root_locus: RootLocus<F>,
    view: View,
    interval: F,
    precision: F,
    rng: Vec<F>,
}

impl<F> Midware<F>
where 
    F: Float + Display + Debug,
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        Self {
            poles: vec![],
            zeros: vec![],
            root_locus: RootLocus::default(),
            view: View::default(),
            interval: F::from(1e-2).unwrap(),
            precision: F::from(1e-6).unwrap(),
            rng: generate_rng(128),
        }
    }

    pub fn poles_mut(&mut self) -> &mut Vec<F> {
        &mut self.poles
    }

    pub fn zeros_mut(&mut self) -> &mut Vec<F> {
        &mut self.zeros
    }

    pub fn update(&mut self, something: ()) {
        self.root_locus.calculate_all(
            self.precision,
            self.interval, 
            F::from(0.01).unwrap(), 
            F::from(1000.0).unwrap(),
            &self.rng
        );

                
    }
}
