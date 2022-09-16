use super::Polynomial;
use approx::{AbsDiffEq, RelativeEq};
use num::{Complex, Float, Num, One, Zero};
use rand::{distributions::Standard, prelude::*};

pub enum PolynomialRoot<F> {
    RealSingle(F),
    ComplexPair(Complex<F>),
}

impl<F: Num + Clone> PolynomialRoot<F> {
    pub fn as_complex(&self) -> Complex<F> {
        match self {
            Self::RealSingle(x) => Complex::from(x.clone()),
            Self::ComplexPair(z) => z.clone(),
        }
    }
}

/// An implementation for finding the complex roots of a univariate polynomial
///
/// As the main objective of this trait is to be used in real-time rendering,
/// efficiency is a must, and is of first priority.
pub trait RootFinding<F: Float> {
    /// Implementation of the Alberth's method
    /// Link to Wikipedia page [here](https://en.wikipedia.org/wiki/Aberth_method)
    ///
    /// Will call `determine_roots_bounds` and generate evenly distributed
    /// start guesses inside that region
    fn find_roots(&self, output: &mut [Complex<F>], prec: F) -> usize;

    /// Implementation of the Alberth's method
    /// Link to Wikipedia page [here](https://en.wikipedia.org/wiki/Aberth_method)
    ///
    /// Receives the initial position for the roots
    fn find_roots_from(&self, initial_guess: &mut [Complex<F>], prec: F) -> usize;

    /// Same as `RootFinding::find_roots_from`, but adds relative randomness to the points
    fn find_roots_from_rand<R>(
        &self,
        initial_guess: &mut [Complex<F>],
        prec: F,
        rng: &mut R,
    ) -> usize
    where
        Standard: Distribution<F>,
        R: RngCore;

    /// Determines lower and upper bounds for the module of the polynomial roots
    ///
    /// Time complexity: same as `determine_max_bound`, which is called twice
    fn determine_roots_bounds(&self) -> (F, F);

    /// Determines the upper bounds by using Lagrange's and Cauchy's bounds.
    /// Returns the smallest of the two.
    ///
    /// Time complexity: $ O(n) $
    fn determine_max_bound(terms: &[F]) -> F {
        let mut lagrange = F::zero();
        let mut cauchy = F::zero();
        let &last = terms.last().unwrap();

        for &term in terms.iter().rev().skip(1) {
            let div = (term / last).abs();
            lagrange = lagrange + div;
            cauchy = (term / last).abs().max(cauchy);
        }

        lagrange = lagrange.max(F::one());
        cauchy = cauchy + F::one();

        lagrange.min(cauchy)
    }
}

impl<F: Float> RootFinding<F> for Polynomial<F> {
    fn find_roots(&self, output: &mut [Complex<F>], prec: F) -> usize {
        debug_assert_eq!(self.order(), output.len());

        let (min, max) = self.determine_roots_bounds();
        let avg = (min + max) / F::from(2).unwrap();

        let angle = F::from(360.0).unwrap() / F::from(self.order()).unwrap();

        output.iter_mut().enumerate().for_each(|(i, out)| {
            // should probably add randomness
            let cpx = Complex::from_polar(avg, F::from(i).unwrap() * angle + F::from(0.5).unwrap());

            *out = cpx;
        });

        self.find_roots_from(output, prec)
    }

    fn find_roots_from_rand<R>(
        &self,
        initial_guess: &mut [Complex<F>],
        prec: F,
        rng: &mut R,
    ) -> usize
    where
        Standard: Distribution<F>,
        R: RngCore,
    {
        for x in initial_guess.iter_mut() {
            let r = Complex::from(F::from(0.01).unwrap()) * Complex::new(rng.gen(), rng.gen());
            *x = r + *x;
        }

        self.find_roots_from(initial_guess, prec)
    }

    fn find_roots_from(&self, guesses: &mut [Complex<F>], prec: F) -> usize {
        let mut max_off = F::infinity();
        let mut count = 0;

        while max_off > prec && count < 50 {
            max_off = F::zero();
            count += 1;

            let mut offsets = vec![Complex::<F>::zero(); guesses.len()];

            for (k, off) in offsets.iter_mut().enumerate() {
                let frac = self.eval_complex(guesses[k]) / self.eval_complex_derivative(guesses[k]);

                let mut sum: Complex<F> = Complex::<F>::zero();
                for (j, guess) in guesses.iter().enumerate() {
                    if j != k {
                        sum = sum + Complex::<F>::one() / (guesses[k] - guess);
                    }
                }

                *off = frac / (Complex::<F>::one() - frac * sum);

                let norm = off.norm_sqr();
                if norm > max_off {
                    max_off = norm;
                }
            }

            guesses
                .iter_mut()
                .zip(offsets)
                .for_each(|(g, o)| *g = *g - o);
        }

        count
    }

    fn determine_roots_bounds(&self) -> (F, F) {
        let terms = self.get_terms();
        let upper = Self::determine_max_bound(terms);
        let rev_terms: Vec<F> = terms.iter().copied().rev().collect();
        let lower = F::one() / Self::determine_max_bound(&rev_terms);

        (lower, upper)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Root<F>(Complex<F>);

impl<F: AbsDiffEq + Float> AbsDiffEq for Root<F> {
    type Epsilon = F;

    fn default_epsilon() -> Self::Epsilon {
        F::epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        F::abs_sub(self.0.re, other.0.re) <= epsilon && F::abs_sub(self.0.im, other.0.im) <= epsilon
    }
}

impl<F: AbsDiffEq<Epsilon = F> + RelativeEq + Float> RelativeEq for Root<F> {
    fn default_max_relative() -> Self::Epsilon {
        F::epsilon()
    }

    fn relative_eq(&self, other: &Self, epsilon: F, max_relative: F) -> bool {
        self.0.re.relative_eq(&other.0.re, epsilon, max_relative)
            && self.0.im.relative_eq(&other.0.im, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use num::complex::Complex64;

    use super::*;

    #[test]
    fn max_bound() {
        let terms = [1.0, 2.0, -8.0, 4.0];
        let roots = [-0.2406, 0.6555, 1.5850];

        let max_bound = Polynomial::determine_max_bound(&terms);

        for root in roots {
            assert!(
                root.abs() <= max_bound,
                "There are roots larger than the max bound"
            );
        }
    }

    #[test]
    fn bounds() {
        let terms = vec![1.0, 2.0, -8.0, 4.0];
        let roots = [-0.2406, 0.6555, 1.5850];

        let (min, max) = Polynomial(terms).determine_roots_bounds();

        for root in roots {
            assert!(
                root.abs() >= min,
                "There are roots smaller than the min bound"
            );
            assert!(
                root.abs() <= max,
                "There are roots larger than the max bound"
            );
        }
    }

    #[test]
    fn find_roots_from() {
        let terms = vec![2.0, 0.0, -1.0, 1.0];

        let mut out = vec![Complex64::zero(); 3];

        Polynomial(terms).find_roots(&mut out, 1e-6);

        let expected = vec![
            Complex64::new(1.0, -1.0),
            Complex64::new(1.0, 1.0),
            Complex64::new(-1.0, 0.0),
        ];

        out.into_iter().zip(expected).for_each(|(a, b)| {
            assert_abs_diff_eq!(Root(a), Root(b), epsilon = 1e-6);
        });
    }
}
