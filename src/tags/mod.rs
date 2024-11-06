#![doc = include_str!("README.md")]
use std::fmt::{self, Debug};

#[cfg(feature = "serde-tags")]
use serde::de::DeserializeOwned;
#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(any(feature = "serde-tags", feature = "serde"))]
use serde::Serialize;

mod error;
mod helpers;
mod predefined_types;
mod svc;

pub use aws_macros::{Tag, Tags};
pub use error::{ParseTagAwsError, ParseTagError, ParseTagValueError, ParseTagsError};

#[derive(Debug, PartialEq, Eq)]
struct InnerTagValue<T>(T)
where
    T: Debug + Clone + PartialEq + Eq + Send;

impl<T> From<T> for InnerTagValue<T>
where
    T: Debug + Clone + PartialEq + Eq + Send,
{
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> From<InnerTagValue<T>> for String
where
    T: Debug + Clone + PartialEq + Eq + Into<RawTagValue> + Send,
{
    fn from(value: InnerTagValue<T>) -> Self {
        value.0.into().0
    }
}

impl<T> PartialEq<T> for InnerTagValue<T>
where
    T: Debug + Clone + PartialEq + Eq + Into<RawTagValue> + Send,
{
    fn eq(&self, other: &T) -> bool {
        &self.0 == other
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RawTagValue(String);
helpers::impl_string_wrapper!(RawTagValue);

#[cfg(feature = "serde-tags")]
pub struct TranslateSerde;
pub struct TranslateManual;

pub trait Translator<S: ?Sized, T> {
    type Error;

    fn from_raw_tag(value: RawTagValue) -> Result<T, Self::Error>;
    fn into_raw_tag(value: T) -> RawTagValue;
}

#[cfg(feature = "serde-tags")]
pub trait TranslatableSerde: Serialize + DeserializeOwned {}
pub trait TranslatableManual:
    TryFrom<RawTagValue, Error: Into<ParseTagValueError>> + Into<RawTagValue>
{
}

pub trait TagValue<V> {
    type Error;
    type Translator: Translator<Self, V, Error = Self::Error>;

    fn from_raw_tag(value: RawTagValue) -> Result<V, Self::Error> {
        Self::Translator::from_raw_tag(value)
    }

    fn into_raw_tag(value: V) -> RawTagValue {
        Self::Translator::into_raw_tag(value)
    }
}

#[cfg(feature = "serde-tags")]
impl<S, T> Translator<S, T> for TranslateSerde
where
    T: TranslatableSerde,
{
    type Error = ParseTagValueError;

    fn from_raw_tag(value: RawTagValue) -> Result<T, Self::Error> {
        serde_json::from_str::<T>(value.as_str()).map_err(|e| ParseTagValueError::InvalidValue {
            value,
            message: e.to_string(),
        })
    }

    fn into_raw_tag(value: T) -> RawTagValue {
        RawTagValue(serde_json::to_string(&value).expect("serialization always succeeds"))
    }
}

impl<S, T> Translator<S, T> for TranslateManual
where
    T: TranslatableManual,
{
    type Error = ParseTagValueError;

    fn from_raw_tag(value: RawTagValue) -> Result<T, Self::Error> {
        match value.clone().try_into() {
            Ok(r) => Ok(r),
            Err(e) => Err(ParseTagValueError::InvalidValue {
                value,
                message: {
                    let e: ParseTagValueError = e.into();
                    e.to_string()
                },
            }),
        }
    }

    fn into_raw_tag(value: T) -> RawTagValue {
        value.into()
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTag {
    key: TagKey,
    value: RawTagValue,
}

impl RawTag {
    pub fn new(key: impl Into<TagKey>, value: impl Into<RawTagValue>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    pub const fn key(&self) -> &TagKey {
        &self.key
    }

    pub const fn value(&self) -> &RawTagValue {
        &self.value
    }
}

impl<T> From<Tag<T>> for RawTag
where
    T: Debug + Clone + PartialEq + Eq + Send,
    T: TagValue<T>,
    InnerTagValue<T>: fmt::Display + Into<RawTagValue>,
{
    fn from(tag: Tag<T>) -> Self {
        Self {
            key: tag.key,
            value: tag.value.into(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TagKey(String);
helpers::impl_string_wrapper!(TagKey);

/// A tag generic over its containing value type.
///
/// There are two ways to construct a `Tag`:
///
/// * You already have a `T`: Just use [`new()`](Self::new())
///
/// ```rust
/// # use aws_lib::tags::Tag;
/// let tag = Tag::<bool>::new("foo".to_owned(), true);
/// ```
///
/// * You have something that may be able to be turned into a [`TagValue<T>`]. In that
///   case, either use [`parse()`](Self::parse()) or the `TryInto` implementation.
///   [`parse()`](Self::parse()) accepts an `impl Into<RawTagValue>`, which is implemented
///   for `String`:
///
/// ```rust
/// # use aws_lib::tags::Tag;
/// let tag = Tag::<bool>::parse("foo".to_owned(), "true".to_owned()).unwrap();
/// ```
///
/// Both the `TryInto` implementation and [`parse()`](Self::parse()) may return a
/// [`ParseTagValueError`] that contains more information about the parse failure.
#[derive(Debug, PartialEq, Eq)]
pub struct Tag<T>
where
    T: Debug + Clone + PartialEq + Eq + Send,
    T: TagValue<T>,
{
    key: TagKey,
    value: InnerTagValue<T>,
}

impl<T> TryFrom<RawTag> for Tag<T>
where
    T: Clone + PartialEq + Eq + Debug + Send,
    T: TagValue<T, Error = ParseTagValueError>,
{
    type Error = ParseTagValueError;

    fn try_from(tag: RawTag) -> Result<Self, Self::Error> {
        Self::parse(tag.key, tag.value)
    }
}

impl<T> Tag<T>
where
    T: Debug + Clone + PartialEq + Eq + Send,
    T: TagValue<T>,
{
    pub fn new(key: impl Into<TagKey>, value: T) -> Self {
        Self {
            key: key.into(),
            value: InnerTagValue(value),
        }
    }

    pub fn parse(
        key: impl Into<TagKey>,
        value: impl Into<String>,
    ) -> Result<Self, ParseTagValueError>
    where
        T: TagValue<T, Error = ParseTagValueError>,
    {
        Ok(Self {
            key: key.into(),
            value: {
                InnerTagValue(<T as TagValue<T>>::from_raw_tag(RawTagValue::new(
                    value.into(),
                ))?)
            },
        })
    }
}

impl<T> Tag<T>
where
    T: Debug + Clone + PartialEq + Eq + Send,
    T: TagValue<T>,
{
    fn into_parts(self) -> (TagKey, InnerTagValue<T>) {
        (self.key, self.value)
    }

    pub const fn key(&self) -> &TagKey {
        &self.key
    }

    pub const fn value(&self) -> &T {
        &self.value.0
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagList(Vec<RawTag>);

impl TagList {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, tag: RawTag) {
        self.0.push(tag);
    }

    pub fn extend(&mut self, tags: Vec<RawTag>) {
        self.0.extend(tags);
    }

    pub fn join(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    pub const fn from_vec(value: Vec<RawTag>) -> Self {
        Self(value)
    }

    pub fn get(&self, key: impl Into<TagKey>) -> Option<&RawTag> {
        let key: TagKey = key.into();
        self.0.iter().find(|tag| tag.key == key)
    }

    pub fn into_vec(self) -> Vec<RawTag> {
        self.0
    }

    pub fn as_slice(&self) -> &[RawTag] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde-tags")]
    use serde::{Deserialize, Serialize};

    use super::*;

    #[Tag(translate = manual)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MyTag {
        A,
        B,
    }

    #[cfg(feature = "serde-tags")]
    #[Tag(translate = serde)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct MyStructTag {
        foo: String,
        bar: bool,
    }

    impl TryFrom<RawTagValue> for MyTag {
        type Error = ParseTagValueError;

        fn try_from(value: RawTagValue) -> Result<Self, Self::Error> {
            match value.as_str() {
                "A" => Ok(Self::A),
                "B" => Ok(Self::B),
                _ => Err(ParseTagValueError::InvalidValue {
                    value,
                    message: "could not match to MyTag enum".to_owned(),
                }),
            }
        }
    }

    impl From<MyTag> for RawTagValue {
        fn from(value: MyTag) -> Self {
            match value {
                MyTag::A => Self("A".to_owned()),
                MyTag::B => Self("B".to_owned()),
            }
        }
    }

    #[test]
    fn use_tag_directly() {
        let key = "Name";

        let tag1 = Tag::<bool>::new(key.to_owned(), true);
        assert!(*tag1.value());

        let tag2 = Tag::<bool>::parse(key.to_owned(), "true".to_owned()).unwrap();
        assert!(*tag2.value());
    }

    #[test]
    fn use_attribute_macro() {
        #[Tags]
        struct MyWrappedTags {
            tag1: String,
            tag2: bool,
            tag3: Option<bool>,
            tag4: Option<bool>,
            #[tag(key = "myname")]
            tag5: MyTag,
            #[tag(key = "anothername")]
            tag6: Option<MyTag>,
            tag7: Option<MyTag>,
            #[cfg(feature = "serde-tags")]
            tag8: MyStructTag,
            #[cfg(feature = "serde-tags")]
            tag9: Option<MyStructTag>,
        }

        let tags = TagList::from_vec(vec![
            RawTag::new("tag1".to_owned(), "false".to_owned()),
            RawTag::new("tag2".to_owned(), "true".to_owned()),
            RawTag::new("tag3".to_owned(), "false".to_owned()),
            RawTag::new("myname".to_owned(), "A".to_owned()),
            RawTag::new("anothername".to_owned(), "B".to_owned()),
            #[cfg(feature = "serde-tags")]
            RawTag::new("tag8".to_owned(), r#"{"foo":"hi","bar":false}"#.to_owned()),
        ]);

        let tags = MyWrappedTags::from_tags(tags).unwrap();

        assert!(tags.tag1 == "false");
        assert!(tags.tag2);
        assert!(tags.tag3 == Some(false));
        assert!(tags.tag4.is_none());
        assert!(tags.tag5 == MyTag::A);
        assert!(tags.tag6 == Some(MyTag::B));
        assert!(tags.tag7.is_none());
        #[cfg(feature = "serde-tags")]
        assert!(
            tags.tag8
                == MyStructTag {
                    foo: "hi".to_owned(),
                    bar: false
                }
        );
        #[cfg(feature = "serde-tags")]
        assert!(tags.tag9.is_none());

        let into_tags = tags.into_tags();

        assert_eq!(
            into_tags,
            TagList::from_vec(vec![
                RawTag::new("tag1".to_owned(), "false".to_owned()),
                RawTag::new("tag2".to_owned(), "true".to_owned()),
                RawTag::new("tag3".to_owned(), "false".to_owned()),
                RawTag::new("myname".to_owned(), "A".to_owned()),
                RawTag::new("anothername".to_owned(), "B".to_owned()),
                #[cfg(feature = "serde-tags")]
                RawTag::new("tag8".to_owned(), r#"{"foo":"hi","bar":false}"#.to_owned(),),
            ])
        );
    }

    #[test]
    fn test_transparent_tag() {
        #[Tag(translate = transparent)]
        #[derive(PartialEq, Debug)]
        struct MyTag(String);

        assert_eq!(
            MyTag::into_raw_tag(MyTag("test".to_owned())),
            RawTagValue::new("test".to_owned())
        );
        assert_eq!(
            MyTag::from_raw_tag(RawTagValue::new("test".to_owned())).unwrap(),
            MyTag("test".to_owned())
        );
    }

    #[test]
    fn test_enums() {
        #[Tag(translate = transparent)]
        #[derive(PartialEq, Debug)]
        enum MyCoolioTag {
            A,
            #[tag(rename = "C")]
            B,
        }

        assert_eq!(
            MyCoolioTag::into_raw_tag(MyCoolioTag::A),
            RawTagValue::new("A".to_owned())
        );
        assert_eq!(
            MyCoolioTag::from_raw_tag(RawTagValue::new("A".to_owned())).unwrap(),
            MyCoolioTag::A
        );

        assert_eq!(
            MyCoolioTag::into_raw_tag(MyCoolioTag::B),
            RawTagValue::new("C".to_owned())
        );
        assert_eq!(
            MyCoolioTag::from_raw_tag(RawTagValue::new("C".to_owned())).unwrap(),
            MyCoolioTag::B
        );
    }
}
