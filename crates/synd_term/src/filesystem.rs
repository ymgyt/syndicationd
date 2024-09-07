#[cfg(test)]
pub(crate) mod mock {
    use std::{
        collections::HashMap,
        fs::File,
        io,
        path::{Path, PathBuf},
    };
    use synd_stdx::fs::FileSystem;

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

    impl FileSystem for MockFileSystem {
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
