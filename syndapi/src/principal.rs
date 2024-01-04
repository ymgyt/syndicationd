#[derive(Clone, Debug)]
pub enum Principal {
    User { email: String },
}
