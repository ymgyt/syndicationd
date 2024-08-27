use std::{fs::File, io, path::Path};

pub trait FileSystem {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;

    fn create_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;

    fn open_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}

pub mod fsimpl {
    use std::{fs::File, io, path::Path};

    #[derive(Debug, Clone, Default)]
    pub struct FileSystem {}

    impl FileSystem {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl super::FileSystem for FileSystem {
        fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
            std::fs::create_dir_all(path)
        }

        fn create_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
            std::fs::File::create(path)
        }

        fn open_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
            std::fs::File::open(path)
        }

        fn remove_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
            std::fs::remove_file(path)
        }
    }
}

#[cfg(test)]
pub(crate) mod mock {
    use std::{
        collections::HashMap,
        fs::File,
        io,
        path::{Path, PathBuf},
    };

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
        fn create_dir_all<P: AsRef<Path>>(&self, _path: P) -> io::Result<()> {
            unimplemented!()
        }

        fn create_file<P: AsRef<Path>>(&self, _path: P) -> io::Result<File> {
            unimplemented!()
        }
        fn open_file<P: AsRef<Path>>(&self, _path: P) -> io::Result<File> {
            unimplemented!()
        }

        fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
            let path = path.as_ref();
            match self.remove_errors.get(path) {
                Some(err) => Err(io::Error::from(*err)),
                None => Ok(()),
            }
        }
    }
}
