
// This implementation of comparable f64 type was copied without
// edit (stolen) from https://github.com/Nadrieril/dhall-rust

#[derive(Debug, Copy, Clone)]
pub struct NaiveDouble(f64);


impl PartialEq for NaiveDouble {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for NaiveDouble {}

impl std::hash::Hash for NaiveDouble {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.0.to_bits().hash(state)
    }
}

impl From<f64> for NaiveDouble {
    fn from(x: f64) -> Self {
        NaiveDouble(x)
    }
}

impl From<NaiveDouble> for f64 {
    fn from(x: NaiveDouble) -> f64 {
        x.0
    }
}