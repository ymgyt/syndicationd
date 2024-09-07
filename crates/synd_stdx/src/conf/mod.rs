/// `Entry` holds candidates for the final value for the configuration.
#[derive(Debug)]
pub struct Entry<T> {
    flag: Option<T>,
    file: Option<T>,
    default: T,
}

impl<T> Entry<T> {
    pub fn with_default(default: T) -> Self {
        Self {
            flag: None,
            file: None,
            default,
        }
    }

    #[must_use]
    pub fn with_file(self, file: Option<T>) -> Self {
        Self { file, ..self }
    }

    #[must_use]
    pub fn with_flag(self, flag: Option<T>) -> Self {
        Self { flag, ..self }
    }

    pub fn resolve_ref(&self) -> &T {
        self.flag
            .as_ref()
            .or(self.file.as_ref())
            .unwrap_or(&self.default)
    }
}

impl<T> Entry<T>
where
    T: Copy,
{
    pub fn resolve(&self) -> T {
        *self.resolve_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_order() {
        let flag = 10;
        let file = 9;
        let default = 8;

        let e = Entry {
            flag: Some(flag),
            file: Some(file),
            default,
        };
        assert_eq!(e.resolve(), flag, "flag should have highest priority");

        let e = Entry {
            flag: None,
            file: Some(file),
            default,
        };
        assert_eq!(
            e.resolve(),
            file,
            "file should have higher priority over default"
        );

        let e = Entry {
            flag: None,
            file: None,
            default,
        };
        assert_eq!(e.resolve(), default);
    }
}
