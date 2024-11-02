Type-safe operations on AWS tags.

# Overview

This basic data structure of this module is the [`TagList`] struct. It contains
a list of untyped AWS tags. It implements `TryFrom` and `Into` for the relevant
`Tag` types in the `aws_sdk_*` crates. In itself, that is not too interesting,
as the tag values are still just `String`.

To get typed tags, there is a [`macro@Tags`] macro. Applied to a struct, it adds
methods to create a struct instance for an instance [`TagList`], and turn a
struct instance back into a [`TagList`]:

```rust
use aws::tags::{Tags, TagList, RawTag};

#[Tags]
struct MyTags {
   tag1: String,
   tag2: bool,
   tag3: Option<bool>,
}

let tags = TagList::from_vec(vec![
  RawTag::new("tag1".to_owned(), "foo".to_owned()),
  RawTag::new("tag2".to_owned(), "true".to_owned()),
]);

let parsed = MyTags::from_tags(tags).unwrap();

assert!(parsed.tag1 == "foo");
assert!(parsed.tag2);
assert!(parsed.tag3.is_none());
```

## Using custom tag types

By default, encoding and decoding of tags is supported for `String` and `bool`
values.

- `String` is encoded as-is
- `bool` is encoded as `true` and `false`

In case you have your own type `T` you want to encode in a tag, there are two
strategies:

- `serde`, which requires `T` to implement `Serialize` and `Deserialize`.
- `manual`, which requires `T` to impelemnt two traits:
  - `impl TryFrom<RawTagValue> for T`: How to (fallibly) create a `T` from the
    value of an tag
  - `impl From<T> for RawTagValue`: How to turn `T` back into the value of a
    tag. This cannot fail.

There is a [`macro@Tag`] macro that selects the strategy, which can then be used
in a struct that is using `#[Tags]`:

```rust
use aws::tags::{Tag, Tags};
use serde::{Serialize, Deserialize};

#[Tag(translate = serde)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MyTag {
    foo: String,
    bar: bool,
}

#[Tags]
struct MyTags {
   foo: MyTag,
}
```
