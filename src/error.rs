use thiserror::Error;

use crate::StageId;

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

#[derive(Error, Debug)]
pub enum RegistrySortingError {
    #[error("Error while parsing Graph. Possibly due to cyclic reference / rules")]
    GraphVisitMissingNodes,

    #[error("Stage '{0:?}' tried to reference stage '{1:?}', but the latter stage does not exist")]
    MissingStage(StageId, StageId),
}

#[derive(Error, Debug)]
pub enum StageError {
    #[error("The given stage has an invalid name")]
    InvalidName,

    #[error("The given stage has no rules associated with it")]
    MissingRules,

    #[error("Tried to insert the stage into the pipeline, but the stage name was already used")]
    Overlapping,
}