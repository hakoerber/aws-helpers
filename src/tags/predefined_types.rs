use super::{
    ParseTagValueError, TagValue, TranslatableManual, TranslatableSerde, TranslateManual,
    TranslateSerde,
};

// Bools can just be handled by serde.
impl TranslatableSerde for bool {}

impl TagValue<Self> for bool {
    type Error = ParseTagValueError;
    type Translator = TranslateSerde;
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
