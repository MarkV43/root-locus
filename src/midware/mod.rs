use core::f32;
use std::fmt::{Debug, Display};

use curves::{Curves, Point};
use num::Float;
use rand::{distributions::Standard, prelude::Distribution};

use crate::polynomials::root_locus::RootLocus;
use crate::polynomials::roots::PolynomialRoot;
use crate::rng::generate_rng;

pub mod curves;

/// Describes the camera's position and zoom
pub struct View {
    _scale: [f32; 2],
    _center: [f32; 2],
}

impl Default for View {
    fn default() -> Self {
        Self {
            _scale: [1.0, 1.0],
            _center: [-0.5, 0.0],
        }
    }
}

/// Stores information about the camera view,
/// algorithm precision, gain invervals,
/// and transfer function being plotted.
pub struct Midware<F: Float> {
    poles: Vec<PolynomialRoot<F>>,
    zeros: Vec<PolynomialRoot<F>>,
    root_locus: RootLocus<F>,
    _view: View,
    interval: F,
    precision: F,
    rng: Vec<F>,
}

impl<F> Default for Midware<F>
where
    F: Float + Display + Debug,
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F> Midware<F>
where
    F: Float + Display + Debug,
    Standard: Distribution<F>,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            poles: vec![],
            zeros: vec![],
            root_locus: RootLocus::default(),
            _view: View::default(),
            interval: F::from(1.001).unwrap(),
            precision: F::from(1e-6).unwrap(),
            rng: generate_rng(128),
        }
    }

    pub fn poles_mut(&mut self) -> &mut Vec<PolynomialRoot<F>> {
        &mut self.poles
    }

    pub fn zeros_mut(&mut self) -> &mut Vec<PolynomialRoot<F>> {
        &mut self.zeros
    }

    pub fn update<const C: usize, const V: usize, P: Point<F>>(
        &mut self,
        curves: &mut Curves<C, V, P, F>,
    ) {
        let (poly_a, poly_b) = self.root_locus.get_polys_mut();

        poly_a.update_roots(F::one(), &self.zeros);
        poly_b.update_roots(F::one(), &self.poles);

        self.root_locus.calculate_all(
            self.precision,
            F::from(0.01).unwrap(),
            F::from(1000.0).unwrap(),
            &self.rng,
            curves,
        );

        let mut min_re = F::infinity();
        let mut min_im = F::infinity();
        let mut max_re = -F::infinity();
        let mut max_im = -F::infinity();

        for p in curves.get_vertices() {
            if p.x() < min_re {
                min_re = p.x();
            }
            if p.x() > max_re {
                max_re = p.x();
            }
            if p.y() < min_im {
                min_im = p.y();
            }
            if p.y() > max_im {
                max_im = p.y();
            }
        }
    }
}
