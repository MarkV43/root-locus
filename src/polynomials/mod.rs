use num::{Complex, Float, Num, ToPrimitive, Zero};
use std::{
    fmt::{Debug, Display},
    iter::{self, repeat},
    ops::{Add, Mul, Sub},
};

use self::roots::PolynomialRoot;

pub mod root_locus;
pub mod roots;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Polynomial<F: Float>(Vec<F>);

pub type Polynomial32 = Polynomial<f32>;
pub type Polynomial64 = Polynomial<f64>;

pub fn conv<N: Num + Copy>(a: &[N], b: &[N], out: &mut [N]) {
    debug_assert!(a.len() + b.len() - 1 <= out.len());

    out.iter_mut().for_each(|x| *x = N::zero());

    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            out[i + j] = out[i + j] + x * y;
        }
    }
}

/// Removes trailing zeros from the end of a slice by returning another slice
#[allow(dead_code)]
fn remove_trailing_zeros<F: Zero>(vec: &[F]) -> &[F] {
    &vec[..=vec.iter().rposition(|x| !x.is_zero()).unwrap_or(0)]
}

/// Removes trailing zeros from the end of a vector by mutating it
pub fn remove_trailing_zeros_vec<F: Zero>(vec: &mut Vec<F>) {
    vec.truncate(vec.iter().rposition(|x| !x.is_zero()).unwrap_or(0) + 1);
}

impl<F: Float> Polynomial<F> {
    #[must_use]
    pub fn new(mut vec: Vec<F>) -> Self {
        remove_trailing_zeros_vec(&mut vec);
        Self(vec)
    }

    /// Calculates the polynomial with the given roots and gain
    ///
    /// P.S.: If complex roots are given, they must have their complex conjugate too, else the computation will fail
    #[must_use]
    #[allow(clippy::trait_duplication_in_bounds)]
    pub fn from_real_roots<I>(gain: F, roots: &[I]) -> Self
    where
        I: Num + Clone,
        Complex<F>: From<I> + From<F> + Clone,
    {
        let mut out = vec![Complex::zero(); roots.len() + 1];
        out[0] = Complex::from(gain);
        let mut out_copy = out.clone();

        for (i, root) in roots.iter().map(|x| Complex::from(x.clone())).enumerate() {
            conv(&out_copy[..=i], &[-root, Complex::from(F::one())], &mut out);

            out_copy.clone_from_slice(&out);
        }

        Self::new(out.iter().map(|x| x.re).collect::<Vec<_>>())
    }

    #[must_use]
    pub fn from_roots(gain: F, roots: &[PolynomialRoot<F>]) -> Self {
        let mut out = vec![F::zero(); roots.len() * 2 + 1];
        out[0] = gain;
        let mut out_copy = out.clone();
        let mut i = 0;

        for root in roots.iter() {
            match root {
                &PolynomialRoot::RealSingle(r) => {
                    conv(&out_copy[..=i], &[-r, F::one()], &mut out);
                    i += 1;
                }
                &PolynomialRoot::ComplexPair(c) => {
                    // (x - a - b i) * (x - a + b i)
                    // x² - a x + b i x - a x + a² - a b i - b i x + a b i + b²
                    //            ^^^^^              ^^^^^   ^^^^^   ^^^^^
                    // x² - (2 a) x + (a² + b²)
                    conv(
                        &out_copy[..=i],
                        &[
                            c.re.powi(2) + c.im.powi(2),
                            F::from(-2.0).unwrap() * c.re,
                            F::one(),
                        ],
                        &mut out,
                    );
                    i += 2;
                }
            }
            out_copy.clone_from_slice(&out);
        }

        Self::new(out)
    }

    /// Creates a polynomial from the sum `x * a + y * b`
    #[must_use]
    pub fn from_sum(x: F, a: &Self, y: F, b: &Self) -> Self {
        if a.0.len() < b.0.len() {
            return Self::from_sum(y, b, x, a);
        }

        let mut out = vec![F::zero(); a.0.len()];

        let zero = F::zero();
        let bi = b.get_terms().iter().chain(repeat(&zero));

        for ((&i, &j), o) in a.get_terms().iter().zip(bi).zip(out.iter_mut()) {
            *o = x * i + y * j;
        }

        Self::new(out)
    }

    #[must_use]
    pub fn from_mul(a: &Self, b: &Self) -> Self {
        let mut out = vec![F::zero(); a.0.len() + b.0.len() - 1];

        conv(&a.0, &b.0, &mut out);

        Self::new(out)
    }

    #[must_use]
    pub fn get_terms(&self) -> &[F] {
        &self.0
    }

    #[must_use]
    pub fn order(&self) -> usize {
        self.0.len() - 1
    }

    pub fn eval(&self, x: F) -> F {
        let mut res = F::zero();

        for (i, &term) in self.0.iter().enumerate() {
            res = res + term * x.powi(i.to_i32().unwrap());
        }

        res
    }

    pub fn eval_complex(&self, x: Complex<F>) -> Complex<F> {
        let mut res = Complex::zero();

        for (i, &term) in self.0.iter().enumerate() {
            res = res + Complex::from(term) * x.powu(i.to_u32().unwrap());
        }

        res
    }

    pub fn eval_derivative(&self, x: F) -> F {
        let mut res = F::zero();

        for (i, &term) in self.0.iter().enumerate() {
            res = res + term * F::from(i).unwrap() * x.powi(i.to_i32().unwrap().saturating_sub(1));
        }

        res
    }

    pub fn eval_complex_derivative(&self, x: Complex<F>) -> Complex<F> {
        let mut res = Complex::zero();

        for (i, &term) in self.0.iter().enumerate() {
            res = res
                + Complex::from(term)
                    * Complex::from(F::from(i).unwrap())
                    * x.powu(i.to_u32().unwrap().saturating_sub(1));
        }

        res
    }

    #[must_use]
    pub fn derivative(&self) -> Self {
        Self::new(
            self.0
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, &v)| F::from(i).unwrap() * v)
                .collect(),
        )
    }

    // TODO: implement Polynomial methods
}

impl<F: Float + Display> Display for Polynomial<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terms = remove_trailing_zeros(self.get_terms());
        for (i, term) in terms.iter().enumerate().rev() {
            write!(f, "{} x^{}", term, i)?;
            if i > 0 {
                write!(f, " + ")?;
            }
        }
        Ok(())
    }
}

// TODO: implement division

impl<F: Float> Add for Polynomial<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let zero = F::zero();
        Self::new(
            if self.order() > rhs.order() {
                rhs.0.iter().chain(iter::repeat(&zero)).zip(self.0)
            } else {
                self.0.iter().chain(iter::repeat(&zero)).zip(rhs.0)
            }
            .map(|(a, b)| *a + b)
            .collect(),
        )
    }
}

impl<F: Float> Mul<F> for Polynomial<F> {
    type Output = Self;

    fn mul(self, rhs: F) -> Self::Output {
        Self::new(self.0.iter().map(|&x| rhs * x).collect())
    }
}

impl<F: Float> Mul for Polynomial<F> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = vec![F::zero(); self.0.len() + rhs.0.len() - 1];

        conv(&self.0, &rhs.0, &mut out);

        Self::Output::new(out)
    }
}

impl<F: Float> Mul for &Polynomial<F> {
    type Output = Polynomial<F>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = vec![F::zero(); self.0.len() + rhs.0.len() - 1];

        conv(&self.0, &rhs.0, &mut out);

        Self::Output::new(out)
    }
}

impl<F: Float> Sub for Polynomial<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let zero = F::zero();
        Self::new(if self.order() > rhs.order() {
            rhs.0
                .iter()
                .chain(iter::repeat(&zero))
                .zip(self.0)
                .map(|(&a, b)| b - a)
                .collect()
        } else {
            self.0
                .iter()
                .chain(iter::repeat(&zero))
                .zip(rhs.0)
                .map(|(&a, b)| a - b)
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::relative_eq;

    #[test]
    fn from_roots() {
        let roots = vec![1.0, 2.0];
        // 1.0 * (x - 1) * (x - 2)
        // x^2 - 3 x + 2
        let poly = Polynomial(vec![2.0, -3.0, 1.0]);

        assert_eq!(Polynomial::from_real_roots(1.0, &roots), poly);
    }

    #[test]
    fn eval_complex_derivative() {
        let terms = vec![1.0, 2.0, -8.0, 4.0];
        let poly = Polynomial(terms);

        let x = Complex::new(2.0, 2.0);

        let der = poly.eval_complex_derivative(x);

        let real_der = Complex::new(-59.0, 2.0);

        #[allow(unused_must_use)]
        {
            relative_eq!(real_der.re, der.re);
            relative_eq!(real_der.im, der.im);
        }
    }

    #[test]
    fn derivative() {
        let poly = Polynomial(vec![1.0, 2.0, -8.0, 4.0]);
        let der = Polynomial(vec![2.0, -16.0, 12.0]);

        assert_eq!(poly.derivative(), der);
    }

    #[test]
    fn add() {
        let a = Polynomial(vec![1.0, 2.0]); // 1 + 2x
        let b = Polynomial(vec![-1.0, 0.0, -3.0]); // -1 - 3x²
        let c = Polynomial(vec![0.0, 2.0, -3.0]); // a + b = 2x - 3x²

        assert_eq!(a + b, c);
    }

    #[test]
    fn sub() {
        let a = Polynomial(vec![1.0, 2.0]); // 1 + 2x
        let b = Polynomial(vec![-1.0, 0.0, -3.0]); // -1 - 3x²
        let c = Polynomial(vec![2.0, 2.0, 3.0]);

        assert_eq!(a - b, c);
    }

    #[test]
    fn mul_float() {
        let a = Polynomial(vec![-1.0, 2.0, -3.0]); // -1 + 2x - 3x²
        let b = Polynomial(vec![-2.0, 4.0, -6.0]); // -2 + 4x - 6x²

        assert_eq!(a * 2.0, b);
    }

    #[test]
    fn mul_poly() {
        let a = Polynomial(vec![1.0, 2.0]); // 1 + 2x
        let b = Polynomial(vec![-1.0, 0.0, -3.0]); // -1 - 3x²
        let c = Polynomial(vec![-1.0, -2.0, -3.0, -6.0]);

        assert_eq!(a * b, c);
    }

    #[test]
    fn remove_trailing_zeros_works() {
        let a = vec![0.0, 2.0, 3.0, 0.0, 4.0, 0.0, 0.0];
        let b = remove_trailing_zeros(&a);

        assert_eq!(b, &[0.0, 2.0, 3.0, 0.0, 4.0]);

        let c = remove_trailing_zeros(b);
        assert_eq!(c, &[0.0, 2.0, 3.0, 0.0, 4.0]);

        let d = vec![0.0, 0.0, 0.0];
        let e = remove_trailing_zeros(&d);

        assert_eq!(e, &[0.0]);
    }

    #[test]
    fn remove_trailing_zeros_vec_works() {
        let mut a = vec![0.0, 2.0, 3.0, 0.0, 4.0, 0.0, 0.0];

        remove_trailing_zeros_vec(&mut a);
        assert_eq!(a, vec![0.0, 2.0, 3.0, 0.0, 4.0]);

        remove_trailing_zeros_vec(&mut a);
        assert_eq!(a, vec![0.0, 2.0, 3.0, 0.0, 4.0]);

        let mut b = vec![0.0, 0.0, 0.0];

        remove_trailing_zeros_vec(&mut b);
        assert_eq!(b, vec![0.0]);
    }
}
