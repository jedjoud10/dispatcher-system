use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorldBorrowError {
    #[error("Resource is not present in the world")]
    NotPresent,

    #[error("The current system does not have access to the resource")]
    InvalidAccess,

    #[error("{0}")]
    BorrowError(core::cell::BorrowError),
}

#[derive(Error, Debug)]
pub enum WorldBorrowMutError {
    #[error("Resource is not present in the world")]
    NotPresent,

    #[error("The current system does not have access to the resource mutably")]
    InvalidAccess,

    #[error("{0}")]
    BorrowMutError(core::cell::BorrowMutError),
}
