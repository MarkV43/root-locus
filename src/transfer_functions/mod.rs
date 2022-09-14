use num::Float;

use crate::polynomials::Polynomial;

#[allow(dead_code)]
pub struct TransferFunction<F: Float> {
    numerator: Polynomial<F>,
    denominator: Polynomial<F>,
}
