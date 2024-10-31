#![deny(clippy::complexity, clippy::style, clippy::perf)]
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_panics_doc)]

use num::Float;

pub mod polynomials;
pub mod transfer_functions;
pub mod midware;
pub mod rng;

#[derive(PartialEq, Debug)]
pub struct NotNanFloat<F: Float>(pub F);

impl<F: Float> NotNanFloat<F> {
    pub fn new(f: F) -> Self {
        debug_assert!(!f.is_nan());
        Self(f)
    }
}

impl<F: Float> Eq for NotNanFloat<F> {}

impl<F: Float> PartialOrd for NotNanFloat<F> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<F: Float> Ord for NotNanFloat<F> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .map_or_else(|| panic!("NotNanFloat was NaN"), |x| x)
    }
}
