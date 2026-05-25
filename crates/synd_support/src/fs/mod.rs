use std::{fs::File, io, path::Path};

pub mod fsimpl;

#[cfg_attr(feature = "mock", mockall::automock)]
pub trait FileSystem {
    #[cfg_attr(feature = "mock", mockall::concretize)]
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;

    #[cfg_attr(feature = "mock", mockall::concretize)]
    fn create_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;

    #[cfg_attr(feature = "mock", mockall::concretize)]
    fn open_file<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;

    #[cfg_attr(feature = "mock", mockall::concretize)]
    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
}
