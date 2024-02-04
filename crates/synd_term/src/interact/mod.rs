#[cfg(not(feature = "integration"))]
mod interactor;
#[cfg(not(feature = "integration"))]
pub use interactor::Interactor;

#[cfg(feature = "integration")]
mod integration_interactor;
#[cfg(feature = "integration")]
pub use integration_interactor::Interactor;
