use std::{marker::PhantomData, ops::Range};

use num::{Complex, Float};

pub trait Point<F: Float>: From<Complex<F>> + Copy {
    fn x(&self) -> F;
    fn y(&self) -> F;
}

pub struct Curves<const C: usize, const V: usize, P, F> {
    vertices: Vec<P>, // len: MAX_CURVE_COUNT * MAX_CURVE_VERTICES
    vertex_count: u32,
    curve_count: u32,
    _pad: PhantomData<F>,
}

impl<const C: usize, const V: usize, P, F> Default for Curves<C, V, P, F>
where
    P: Point<F> + Default,
    F: Float,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const C: usize, const V: usize, P, F> Curves<C, V, P, F>
where
    P: Point<F>,
    F: Float,
{
    #[must_use]
    pub fn new() -> Self
    where
        P: Default,
    {
        Self {
            vertices: vec![P::default(); C * V],
            vertex_count: 0,
            curve_count: 0,
            _pad: PhantomData,
        }
    }

    pub fn get_indices(data: &mut [u16]) {
        debug_assert_eq!(data.len(), C * (V - 1) * 2);

        let mut v = 0;
        for c in 0..C {
            for k in 0..V - 1 {
                data[(c * (V - 1) + k) * 2] = v;
                data[(c * (V - 1) + k) * 2 + 1] = v + 1;
                v += 1;
            }
            v += 1;
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn get_index_ranges(&self, data: &mut [Option<Range<u32>>]) {
        debug_assert_eq!(data.len(), C);

        for (c, d) in data.iter_mut().enumerate().take(C) {
            *d = if (c as u32) < self.curve_count {
                let start = (c * (V - 1) * 2) as u32;
                let end = start + (self.vertex_count - 1) * 2;
                Some(start..end)
            } else {
                None
            }
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn get_vertex_ranges(&self, data: &mut [Option<Range<usize>>]) {
        debug_assert_eq!(data.len(), C);

        for (c, d) in data.iter_mut().enumerate().take(C) {
            *d = if (c as u32) < self.curve_count {
                let start = c * V;
                let end = start + self.vertex_count as usize;
                Some(start..end)
            } else {
                None
            }
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn write(&mut self, data: &[Vec<P>]) {
        self.curve_count = data.len() as u32;

        if self.curve_count == 0 {
            self.vertex_count = 0;
            return;
        }

        self.vertex_count = data[0].len() as u32;
        assert_ne!(self.vertex_count, 0);

        println!(
            "Curve count: {}\nVertex count: {}",
            self.curve_count, self.vertex_count
        );

        for (i, curve) in data.iter().enumerate() {
            for (j, vert) in curve.iter().enumerate() {
                self.vertices[V * i + j] = *vert;
            }
        }
    }

    #[must_use]
    pub fn get_vertices(&self) -> &[P] {
        &self.vertices
    }

    #[must_use]
    pub fn get_vertices_mut(&mut self) -> &mut [P] {
        &mut self.vertices
    }
}
