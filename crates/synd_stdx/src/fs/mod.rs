use std::{fs::File, io, path::Path};

pub mod fsimpl;

pub trait FileSystem {
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;

    fn create_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;

    fn open_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}
