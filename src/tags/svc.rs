mod ec2 {
    use std::fmt::Debug;

    use super::super::{
        error::ParseTagAwsError, ParseTagError, ParseTagsError, RawTag, RawTagValue, Tag, TagKey,
        TagList, TagValue,
    };

    impl<T> From<Tag<T>> for aws_sdk_ec2::types::Tag
    where
        T: Debug + Clone + PartialEq + Eq + Into<String> + Send,
        T: TagValue<T>,
    {
        fn from(tag: Tag<T>) -> Self {
            let (key, value) = tag.into_parts();
            Self::builder().key(key).value(value.0).build()
        }
    }

    impl From<RawTag> for aws_sdk_ec2::types::Tag {
        fn from(tag: RawTag) -> Self {
            Self::builder().key(tag.key).value(tag.value.0).build()
        }
    }

    impl TryFrom<Vec<aws_sdk_ec2::types::Tag>> for TagList {
        type Error = ParseTagsError;

        fn try_from(list: Vec<aws_sdk_ec2::types::Tag>) -> Result<Self, Self::Error> {
            Ok(Self(
                list.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, ParseTagError>>()?,
            ))
        }
    }

    impl From<TagList> for Vec<aws_sdk_ec2::types::Tag> {
        fn from(tags: TagList) -> Self {
            tags.0.into_iter().map(Into::into).collect()
        }
    }

    impl From<TagList> for Vec<aws_sdk_ec2::types::Filter> {
        fn from(tags: TagList) -> Self {
            tags.0
                .into_iter()
                .map(|tag| {
                    aws_sdk_ec2::types::Filter::builder()
                        .name(format!("tag:{}", tag.key))
                        .values(tag.value)
                        .build()
                })
                .collect()
        }
    }

    impl TryFrom<aws_sdk_ec2::types::Tag> for RawTag {
        type Error = ParseTagError;

        fn try_from(tag: aws_sdk_ec2::types::Tag) -> Result<Self, Self::Error> {
            let key = TagKey(tag.key.ok_or(ParseTagAwsError::AwsKeyNone)?);
            let value = RawTagValue(
                tag.value
                    .ok_or_else(|| ParseTagAwsError::AwsValueNone { key: key.clone() })?,
            );
            Ok(Self { key, value })
        }
    }

    impl PartialEq<aws_sdk_ec2::types::Tag> for RawTag {
        fn eq(&self, other: &aws_sdk_ec2::types::Tag) -> bool {
            Some(&self.key.0) == other.key.as_ref() && Some(&self.value.0) == other.value.as_ref()
        }
    }

    impl PartialEq<RawTag> for aws_sdk_ec2::types::Tag {
        fn eq(&self, other: &RawTag) -> bool {
            other.eq(self)
        }
    }

    impl From<TagList> for aws_sdk_ec2::types::TagSpecification {
        fn from(value: TagList) -> Self {
            Self::builder()
                .resource_type(aws_sdk_ec2::types::ResourceType::Instance)
                .set_tags(Some(value.into()))
                .build()
        }
    }
}

mod cloudformation {
    use std::fmt::Debug;

    use super::super::{
        error::ParseTagAwsError, ParseTagError, ParseTagsError, RawTag, RawTagValue, Tag, TagKey,
        TagList, TagValue,
    };

    impl<T> From<Tag<T>> for aws_sdk_cloudformation::types::Tag
    where
        T: Debug + Clone + PartialEq + Eq + Into<String> + Send,
        T: TagValue<T>,
    {
        fn from(tag: Tag<T>) -> Self {
            let (key, value) = tag.into_parts();
            Self::builder().key(key).value(value.0).build()
        }
    }

    impl From<RawTag> for aws_sdk_cloudformation::types::Tag {
        fn from(tag: RawTag) -> Self {
            Self::builder().key(tag.key).value(tag.value.0).build()
        }
    }

    impl TryFrom<Vec<aws_sdk_cloudformation::types::Tag>> for TagList {
        type Error = ParseTagsError;

        fn try_from(list: Vec<aws_sdk_cloudformation::types::Tag>) -> Result<Self, Self::Error> {
            Ok(Self(
                list.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, ParseTagError>>()?,
            ))
        }
    }

    impl From<TagList> for Vec<aws_sdk_cloudformation::types::Tag> {
        fn from(tags: TagList) -> Self {
            tags.0.into_iter().map(Into::into).collect()
        }
    }

    impl TryFrom<aws_sdk_cloudformation::types::Tag> for RawTag {
        type Error = ParseTagError;

        fn try_from(tag: aws_sdk_cloudformation::types::Tag) -> Result<Self, Self::Error> {
            let key = TagKey(tag.key.ok_or(ParseTagAwsError::AwsKeyNone)?);
            let value = RawTagValue(
                tag.value
                    .ok_or_else(|| ParseTagAwsError::AwsValueNone { key: key.clone() })?,
            );
            Ok(Self { key, value })
        }
    }

    impl PartialEq<aws_sdk_cloudformation::types::Tag> for RawTag {
        fn eq(&self, other: &aws_sdk_cloudformation::types::Tag) -> bool {
            Some(&self.key.0) == other.key.as_ref() && Some(&self.value.0) == other.value.as_ref()
        }
    }

    impl PartialEq<RawTag> for aws_sdk_cloudformation::types::Tag {
        fn eq(&self, other: &RawTag) -> bool {
            other.eq(self)
        }
    }
}

mod efs {
    use std::fmt::Debug;

    use super::super::{
        ParseTagError, ParseTagsError, RawTag, RawTagValue, Tag, TagKey, TagList, TagValue,
    };

    impl<T> From<Tag<T>> for aws_sdk_efs::types::Tag
    where
        T: Debug + Clone + PartialEq + Eq + Into<String> + Send,
        T: TagValue<T>,
    {
        fn from(tag: Tag<T>) -> Self {
            let (key, value) = tag.into_parts();
            Self::builder()
                .key(key)
                .value(value.0)
                .build()
                .expect("builder misused")
        }
    }

    impl From<RawTag> for aws_sdk_efs::types::Tag {
        fn from(tag: RawTag) -> Self {
            Self::builder()
                .key(tag.key)
                .value(tag.value.0)
                .build()
                .expect("builder misused")
        }
    }

    impl TryFrom<Vec<aws_sdk_efs::types::Tag>> for TagList {
        type Error = ParseTagsError;

        fn try_from(list: Vec<aws_sdk_efs::types::Tag>) -> Result<Self, Self::Error> {
            Ok(Self(
                list.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, ParseTagError>>()?,
            ))
        }
    }

    impl From<TagList> for Vec<aws_sdk_efs::types::Tag> {
        fn from(tags: TagList) -> Self {
            tags.0.into_iter().map(Into::into).collect()
        }
    }

    impl TryFrom<aws_sdk_efs::types::Tag> for RawTag {
        type Error = ParseTagError;

        fn try_from(tag: aws_sdk_efs::types::Tag) -> Result<Self, Self::Error> {
            let key = TagKey(tag.key);
            let value = RawTagValue(tag.value);
            Ok(Self { key, value })
        }
    }

    impl PartialEq<aws_sdk_efs::types::Tag> for RawTag {
        fn eq(&self, other: &aws_sdk_efs::types::Tag) -> bool {
            self.key.0 == other.key && self.value.0 == other.value
        }
    }

    impl PartialEq<RawTag> for aws_sdk_efs::types::Tag {
        fn eq(&self, other: &RawTag) -> bool {
            other.eq(self)
        }
    }
}
