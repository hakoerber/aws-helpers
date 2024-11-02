#![expect(clippy::module_name_repetitions, reason = "error prefix is necessary")]

use std::{convert::Infallible, fmt};

use super::{RawTagValue, TagKey};

#[derive(Debug, Clone)]
pub enum ParseTagAwsError {
    AwsKeyNone,
    AwsValueNone { key: TagKey },
}

impl std::error::Error for ParseTagAwsError {}

impl fmt::Display for ParseTagAwsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::AwsKeyNone => write!(f, "aws responded with `None` value for tag key"),
            Self::AwsValueNone { ref key } => write!(
                f,
                "aws responded with `None` value for tag value of tag \"{key}\""
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseTagValueError {
    /// A generic error for type conversions of a tag value to some type `T`
    InvalidValue {
        value: RawTagValue,
        message: String,
    },
    /// like `InvalidValue`, but specific for `bool`
    InvalidBoolValue {
        value: RawTagValue,
    },
    Aws(ParseTagAwsError),
}

impl std::error::Error for ParseTagValueError {}

impl fmt::Display for ParseTagValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidBoolValue { ref value } => {
                write!(f, "invalid tag bool value \"{value}\"")
            }
            Self::InvalidValue {
                ref value,
                ref message,
            } => write!(f, "invalid tag value \"{value}\": {message}"),
            Self::Aws(ref inner) => write!(f, "aws error: {inner}"),
        }
    }
}

impl From<Infallible> for ParseTagValueError {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Debug, Clone)]
/// Like [`ParseTagValueError`], but contains potential additional information about the
/// tag *key*.
pub enum ParseTagError {
    InvalidTagValue {
        key: TagKey,
        inner: ParseTagValueError,
    },
    Aws(ParseTagAwsError),
}

impl std::error::Error for ParseTagError {}

impl fmt::Display for ParseTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Aws(ref inner) => write!(f, "aws error: {inner}"),
            Self::InvalidTagValue { ref key, ref inner } => {
                write!(f, "failed parsing tag \"{key}\": {inner}")
            }
        }
    }
}

/// this is required because `String` can be the inner value of a `Tag`, but is also the input
/// to the `parse()` function. `try_into()` from `String` to `String` returns `Infallible` as
/// the `Err` type.
impl From<Infallible> for ParseTagError {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}

impl From<ParseTagAwsError> for ParseTagError {
    fn from(value: ParseTagAwsError) -> Self {
        Self::Aws(value)
    }
}

#[derive(Debug, Clone)]
/// Errors that can happen when parsing a set of tags.
pub enum ParseTagsError {
    /// A required tag was not found
    TagNotFound { key: TagKey },
    /// A single tag failed to parse
    ParseTag(ParseTagError),
}

impl std::error::Error for ParseTagsError {}

impl fmt::Display for ParseTagsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TagNotFound { ref key } => write!(f, "tag {key} not found in input"),
            Self::ParseTag(ref err) => write!(f, "failed parsing tag: {err}"),
        }
    }
}

impl From<ParseTagError> for ParseTagsError {
    fn from(value: ParseTagError) -> Self {
        Self::ParseTag(value)
    }
}
