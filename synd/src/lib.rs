#[derive(Debug, Clone)]
pub struct Feed {
    pub url: String,
}

impl Feed {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}
