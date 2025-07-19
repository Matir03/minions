pub trait Sigmoid {
    fn sigmoid(self) -> Self;
}

impl Sigmoid for f32 {
    fn sigmoid(self) -> Self {
        1.0 / (1.0 + (-self).exp())
    }
}

impl Sigmoid for f64 {
    fn sigmoid(self) -> Self {
        1.0 / (1.0 + (-self).exp())
    }
}
