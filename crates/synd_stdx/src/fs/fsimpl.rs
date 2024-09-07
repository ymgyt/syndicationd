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
