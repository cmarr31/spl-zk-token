use curve25519_dalek::ristretto::RistrettoPoint;
use digest::{ExtendableOutput, Input, XofReader};
use sha3::{Sha3XofReader, Sha3_512, Shake256};

/// Generators for Pedersen vector commitments.
///
/// The code is copied from https://github.com/dalek-cryptography/bulletproofs for now...

struct GeneratorsChain {
    reader: Sha3XofReader,
}

impl GeneratorsChain {
    /// Creates a chain of generators, determined by the hash of `label`.
    fn new(label: &[u8]) -> Self {
        let mut shake = Shake256::default();
        shake.input(b"GeneratorsChain");
        shake.input(label);

        GeneratorsChain {
            reader: shake.xof_result(),
        }
    }

    /// Advances the reader n times, squeezing and discarding
    /// the result.
    fn fast_forward(mut self, n: usize) -> Self {
        for _ in 0..n {
            let mut buf = [0u8; 64];
            self.reader.read(&mut buf);
        }
        self
    }
}

impl Default for GeneratorsChain {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl Iterator for GeneratorsChain {
    type Item = RistrettoPoint;

    fn next(&mut self) -> Option<Self::Item> {
        let mut uniform_bytes = [0u8; 64];
        self.reader.read(&mut uniform_bytes);

        Some(RistrettoPoint::from_uniform_bytes(&uniform_bytes))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::max_value(), None)
    }
}

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct BulletproofGens {
    /// The maximum number of usable generators.
    pub gens_capacity: usize,
    /// Precomputed \\(\mathbf G\\) generators.
    G_vec: Vec<RistrettoPoint>,
    /// Precomputed \\(\mathbf H\\) generators.
    H_vec: Vec<RistrettoPoint>,
}

impl BulletproofGens {
    pub fn new(gens_capacity: usize) -> Self {
        let mut gens = BulletproofGens {
            gens_capacity: 0,
            G_vec: Vec::new(),
            H_vec: Vec::new(),
        };
        gens.increase_capacity(gens_capacity);
        gens
    }

    // pub fn new_aggregate(gens_capacities: Vec<usize>) -> Vec<BulletproofGens> {
    //     let mut gens_vector = Vec::new();
    //     for (capacity, i) in gens_capacities.iter().enumerate() {
    //         gens_vector.push(BulletproofGens::new(capacity, &i.to_le_bytes()));
    //     }
    //     gens_vector
    // }

    /// Increases the generators' capacity to the amount specified.
    /// If less than or equal to the current capacity, does nothing.
    pub fn increase_capacity(&mut self, new_capacity: usize) {
        if self.gens_capacity >= new_capacity {
            return;
        }

        let mut label = [b'G'];
        self.G_vec.extend(
            &mut GeneratorsChain::new(&[label, [b'G']].concat())
                .fast_forward(self.gens_capacity)
                .take(new_capacity - self.gens_capacity),
        );

        self.H_vec.extend(
            &mut GeneratorsChain::new(&[label, [b'H']].concat())
                .fast_forward(self.gens_capacity)
                .take(new_capacity - self.gens_capacity),
        );

        self.gens_capacity = new_capacity;
    }

    pub(crate) fn G(&self, n: usize) -> impl Iterator<Item = &RistrettoPoint> {
        GensIter {
            array: &self.G_vec,
            n,
            gen_idx: 0,
        }
    }

    pub(crate) fn H(&self, n: usize) -> impl Iterator<Item = &RistrettoPoint> {
        GensIter {
            array: &self.H_vec,
            n,
            gen_idx: 0,
        }
    }
}

struct GensIter<'a> {
    array: &'a Vec<RistrettoPoint>,
    n: usize,
    gen_idx: usize,
}

impl<'a> Iterator for GensIter<'a> {
    type Item = &'a RistrettoPoint;

    fn next(&mut self) -> Option<Self::Item> {
        if self.gen_idx >= self.n {
            None
        } else {
            let cur_gen = self.gen_idx;
            self.gen_idx += 1;
            Some(&self.array[cur_gen])
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.n - self.gen_idx;
        (size, Some(size))
    }
}
