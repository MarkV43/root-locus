use rand::{distributions::Standard, prelude::*};

#[allow(clippy::module_name_repetitions, unused)]
#[must_use]
pub fn generate_rng<T>(count: usize) -> Vec<T>
where
    Standard: Distribution<T>,
{
    let mut vec = Vec::with_capacity(count);
    let mut rng = thread_rng();

    for _ in 0..count {
        vec.push(rng.gen());
    }

    assert_eq!(count, vec.len());

    vec
}
