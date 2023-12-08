use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng,
    thread_rng,
};

pub struct RandomGen<const N: usize> {
    rng: ThreadRng,
    dis: Uniform<usize>,
}

impl<const N: usize> Default for RandomGen<N> {
    fn default() -> Self {
        RandomGen {
            rng: thread_rng(),
            dis: Uniform::new(0, N),
        }
    }
}

impl<const N: usize> RandomGen<N> {
    pub fn gen(&mut self) -> usize {
        self.dis.sample(&mut self.rng)
    }
}
