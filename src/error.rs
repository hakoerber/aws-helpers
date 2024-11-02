use std::{fmt, net, time::Duration};

use crate::tags::{ParseTagError, ParseTagsError};

#[derive(Debug)]
pub enum Error {
    UnexpectedNoneValue {
        entity: String,
    },
    SdkError(Box<dyn std::error::Error + Send>),
    InvalidResponseError {
        message: String,
    },
    MultipleMatches {
        entity: String,
    },
    InvalidTag(ParseTagError),
    InvalidTags(ParseTagsError),
    RunInstancesEmptyResponse,
    InstanceStopExceededMaxWait {
        max_wait: Duration,
        instance: super::InstanceId,
    },
    WaitError(Box<dyn std::error::Error + Send>),
    RunInstanceNoCapacity,
    InvalidTimestampError {
        value: String,
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidTag(ref inner) => {
                write!(f, "{inner}")
            }
            Self::InvalidTags(ref inner) => {
                write!(f, "{inner}")
            }
            Self::UnexpectedNoneValue { ref entity } => {
                write!(f, "entity \"{entity}\" was empty")
            }
            Self::SdkError(ref e) => write!(f, "sdk error: {e}"),
            Self::InvalidResponseError { ref message } => {
                write!(f, "invalid api response: {message}")
            }
            Self::MultipleMatches { ref entity } => {
                write!(f, "multiple matches for {entity} found")
            }
            Self::RunInstancesEmptyResponse => {
                write!(f, "empty instance response of RunInstances")
            }
            Self::InstanceStopExceededMaxWait {
                ref max_wait,
                ref instance,
            } => {
                write!(
                    f,
                    "instance {instance} did not wait in {} seconds",
                    max_wait.as_secs()
                )
            }
            Self::WaitError(ref e) => write!(f, "waiter error: {e}"),
            Self::RunInstanceNoCapacity => {
                write!(f, "no capacity for rnu instance operation")
            }
            Self::InvalidTimestampError {
                ref value,
                ref message,
            } => {
                write!(f, "failed parsing \"{value}\" as timestamp: {message}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl<T: std::error::Error + Send + 'static> From<aws_sdk_ec2::error::SdkError<T>> for Error {
    fn from(value: aws_sdk_ec2::error::SdkError<T>) -> Self {
        Self::SdkError(Box::new(value))
    }
}

impl From<aws_sdk_ec2::waiters::instance_stopped::WaitUntilInstanceStoppedError> for Error {
    fn from(value: aws_sdk_ec2::waiters::instance_stopped::WaitUntilInstanceStoppedError) -> Self {
        Self::WaitError(Box::new(value))
    }
}

impl From<net::AddrParseError> for Error {
    fn from(value: net::AddrParseError) -> Self {
        Self::InvalidResponseError {
            message: value.to_string(),
        }
    }
}

impl From<ParseTagError> for Error {
    fn from(value: ParseTagError) -> Self {
        Self::InvalidTag(value)
    }
}

impl From<ParseTagsError> for Error {
    fn from(value: ParseTagsError) -> Self {
        Self::InvalidTags(value)
    }
}
