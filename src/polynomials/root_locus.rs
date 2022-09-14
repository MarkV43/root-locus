use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use num::{Complex, Float};
use rand::{distributions::Standard, prelude::Distribution, rngs::StdRng, SeedableRng};

use crate::{
    polynomials::{roots::RootFinding, Polynomial},
    NotNanFloat,
};

/// A struct for calculating the roots of a polynomial A + k B,
/// where A == `poly_a` and B == `poly_b` as k varies from 0 to infinity
pub struct RootLocus<F: Float> {
    poly_a: Polynomial<F>,
    poly_b: Polynomial<F>,
    gains: BTreeMap<NotNanFloat<F>, usize>, // map from gain to index in `roots`
    roots: Vec<Complex<F>>,                 // single vec with all roots
    branches: usize,
}

impl<F: Float + Debug> RootLocus<F>
where
    Standard: Distribution<F>,
{
    #[must_use]
    pub fn new(poly_a: Polynomial<F>, poly_b: Polynomial<F>) -> Self {
        // We need A to have higher order
        debug_assert!(poly_a.order() >= poly_b.order());

        let branches = poly_a.order();
        Self {
            poly_a,
            poly_b,
            gains: BTreeMap::new(),
            roots: Vec::new(),
            branches,
        }
    }

    #[must_use]
    pub const fn get_branches(&self) -> usize {
        self.branches
    }

    /// Computes the gain `k` for a given `p` in `A(p) + k B(p) = 0`
    ///
    /// k = - A(p) / B(p)
    pub fn compute_gain(&self, position: Complex<F>) -> Complex<F> {
        -self.poly_a.eval_complex(position) / self.poly_b.eval_complex(position)
    }

    pub fn calculate_all(&mut self, prec: F, interval: F, min_gain: F, max_gain: F) {
        // Add the first point
        self.roots.resize(self.branches, Complex::from(F::zero()));
        // First of all calculate for k == 0.0
        self.gains
            .insert(NotNanFloat::new(F::from(0.0).unwrap()), 0);
        self.poly_a
            .find_roots(&mut self.roots[..self.branches], prec);

        // gains to calculate
        let mut future_gains = BTreeSet::new();

        // TODO Possibly these two clones are bad for performance
        let intersections_poly = self.poly_a.derivative() * self.poly_b.clone()
            - self.poly_b.derivative() * self.poly_a.clone();

        let mut intersections = vec![Complex::from(F::zero()); intersections_poly.order()];
        intersections_poly.find_roots(&mut intersections, prec);

        intersections
            .into_iter()
            .map(|x| self.compute_gain(x))
            .for_each(|x| {
                future_gains.insert(NotNanFloat::new(x.re));
            });

        // drop(intersections);

        let mut k = min_gain;
        while k < max_gain {
            future_gains.insert(NotNanFloat::new(k));
            k = k * interval;
        }

        // Given the size of future_gains, resize self.roots again
        self.roots.resize(
            (1 + future_gains.len()) * self.branches,
            Complex::from(F::zero()),
        );

        let mut old_roots = vec![Complex::from(F::zero()); self.branches];
        old_roots.copy_from_slice(&self.roots[..self.branches]);

        let mut rng = StdRng::seed_from_u64(4343);
        // let mut rng = thread_rng();

        for (i, gain) in future_gains.iter().enumerate() {
            let poly = Polynomial::from_sum(F::one(), &self.poly_a, gain.0, &self.poly_b);

            poly.find_roots_from_rand(&mut old_roots, prec, &mut rng);

            self.roots[(i + 1) * self.branches..(i + 2) * self.branches]
                .copy_from_slice(&old_roots);
        }

        // println!("{:#?}", self.roots);

        // println!("Future gains: {}", future_gains.len());

        // let mut gain = F::from(10.0).unwrap(); // Geometric mean between 0.01 and 10_000
        // future_gains.

        // Calculate for infinite gain
        // We can approximate all roots and calculate the exact location of the finite roots
        // And then replace the approximated values for the exact ones, in their respective positions
        // These steps should be necessary to keep the roots order
    }

    #[must_use]
    pub fn get_roots(&self) -> &[Complex<F>] {
        &self.roots
    }

    #[must_use]
    pub const fn get_gains(&self) -> &BTreeMap<NotNanFloat<F>, usize> {
        &self.gains
    }
}
