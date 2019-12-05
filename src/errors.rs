use rusoto_core::{RusotoError};
use rusoto_dynamodb::{GetItemError, PutItemError, QueryError};
use rusoto_lambda::{InvokeAsyncError};

#[derive(Debug)]
pub enum RegulatorsError {
    GetItemError(RusotoError<GetItemError>),
    PutItemError(RusotoError<PutItemError>),
    QueryError(RusotoError<QueryError>),
    InvokeAsyncError(RusotoError<InvokeAsyncError>),
    SerdeError(serde_dynamodb::Error),
}

impl From<serde_dynamodb::Error> for RegulatorsError {
    fn from(se: serde_dynamodb::Error) -> Self {
        RegulatorsError::SerdeError(se)
    }
}

impl From<RusotoError<PutItemError>> for RegulatorsError {
    fn from(re: RusotoError<PutItemError>) -> Self {
        RegulatorsError::PutItemError(re)
    }
}

impl From<RusotoError<QueryError>> for RegulatorsError {
    fn from(re: RusotoError<QueryError>) -> Self {
        RegulatorsError::QueryError(re)
    }
}

impl From<RusotoError<GetItemError>> for RegulatorsError {
    fn from(re: RusotoError<GetItemError>) -> Self {
        RegulatorsError::GetItemError(re)
    }
}

impl From<RusotoError<InvokeAsyncError>> for RegulatorsError {
    fn from(re: RusotoError<InvokeAsyncError>) -> Self {
        RegulatorsError::InvokeAsyncError(re)
    }
}
