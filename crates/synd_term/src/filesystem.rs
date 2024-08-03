use std::{io, path::Path};

pub trait FileSystem {
    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}

pub mod fsimpl {
    #[derive(Debug, Clone)]
    pub struct FileSystem {}

    impl FileSystem {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl super::FileSystem for FileSystem {
        fn remove_file<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
            std::fs::remove_file(path)
        }
    }
}

#[cfg(test)]
pub(crate) mod mock {
    use std::{collections::HashMap, io, path::PathBuf};

    #[derive(Default, Clone)]
    pub(crate) struct MockFileSystem {
        remove_errors: HashMap<PathBuf, io::ErrorKind>,
    }

    impl MockFileSystem {
        pub(crate) fn with_remove_errors(
            mut self,
            path: impl Into<PathBuf>,
            err: io::ErrorKind,
        ) -> Self {
            self.remove_errors.insert(path.into(), err);
            self
        }
    }

    impl super::FileSystem for MockFileSystem {
        fn remove_file<P: AsRef<std::path::Path>>(&self, path: P) -> io::Result<()> {
            let path = path.as_ref();
            match self.remove_errors.get(path) {
                Some(err) => Err(io::Error::from(*err)),
                None => Ok(()),
            }
        }
    }
}
