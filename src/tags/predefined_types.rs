use super::{ParseTagValueError, RawTagValue, TagValue, TranslatableManual, TranslateManual};

impl TranslatableManual for bool {}

const TRUE_STR: &str = "true";
const FALSE_STR: &str = "false";

impl TagValue<Self> for bool {
    type Error = ParseTagValueError;
    type Translator = TranslateManual;
}

impl TryFrom<RawTagValue> for bool {
    type Error = ParseTagValueError;

    fn try_from(value: RawTagValue) -> Result<Self, Self::Error> {
        match value.as_str() {
            TRUE_STR => Ok(true),
            FALSE_STR => Ok(false),
            _ => Err(ParseTagValueError::InvalidBoolValue { value }),
        }
    }
}

impl From<bool> for RawTagValue {
    fn from(value: bool) -> Self {
        if value {
            Self::new(TRUE_STR.to_owned())
        } else {
            Self::new(FALSE_STR.to_owned())
        }
    }
}

// Due to quoting, we cannot use serde here. It would produce quoted
// strings. Instead, we just serialize/deserialize strings as-is.
// Note that we get the `String` conversion functions for free via the
// `impl_string_wrapper` macro application on `RawTagValue`.
impl TranslatableManual for String {}

impl TagValue<Self> for String {
    type Error = ParseTagValueError;
    type Translator = TranslateManual;
}
