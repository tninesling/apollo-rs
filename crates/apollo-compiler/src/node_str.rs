#![allow(unstable_name_collisions)] // for `sptr::Strict`
use crate::execution::GraphQLLocation;
use crate::NodeLocation;
use crate::SourceMap;
use std::sync::Arc;

/// Smart string type for names and string values in a GraphQL document
///
/// Like [`Node`][crate::Node] it is thread-safe, reference-counted,
/// and carries an optional source location.
/// It is a thin pointer to a single allocation, with a header followed by string data.
#[derive(Clone)]
pub enum NodeStr {
    /// A string stored in the heap, with an optional source location
    Heap(Arc<str>, Option<NodeLocation>),
    /// A static string
    Static(&'static str),
}

impl NodeStr {
    /// Create a new `NodeStr` parsed from the given source location
    #[inline]
    pub fn new_parsed(value: &str, location: NodeLocation) -> Self {
        Self::Heap(value.into(), Some(location))
    }
    /// Create a new `NodeStr` programatically, not parsed from a source file
    #[inline]
    pub fn new(value: &str) -> Self {
        Self::Heap(value.into(), None)
    }
    /// Create a new `NodeStr` from a static string.
    ///
    /// `&str` is a wide pointer (length as pointer metadata stored next to the data pointer),
    /// but we only have space for a thin pointer. So add another `&_` indirection.
    ///
    /// Example:
    ///
    /// ```
    /// let s = apollo_compiler::NodeStr::from_static(&"example");
    /// assert_eq!(s, "example");
    /// ```
    #[inline]
    pub const fn from_static(str_ref: &'static &'static str) -> Self {
        Self::Static(*str_ref)
    }
    #[inline]
    pub fn location(&self) -> Option<NodeLocation> {
        match self {
            Self::Heap(_, location) => *location,
            _ => None,
        }
    }
    /// If this string contains a location, convert it to line and column numbers
    pub fn line_column(&self, sources: &SourceMap) -> Option<GraphQLLocation> {
        match self {
            Self::Heap(_, location) => GraphQLLocation::from_node(sources, *location),
            _ => None,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Heap(heap, _) => heap.as_ref(),
            Self::Static(static_str) => *static_str,
        }
    }
}
impl std::hash::Hash for NodeStr {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state) // location not included
    }
}
impl std::ops::Deref for NodeStr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
impl AsRef<str> for NodeStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl std::borrow::Borrow<str> for NodeStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Debug for NodeStr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl std::fmt::Display for NodeStr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl Eq for NodeStr {}
impl PartialEq for NodeStr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_ref() // don’t compare location
    }
}
impl Ord for NodeStr {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}
impl PartialOrd for NodeStr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq<str> for NodeStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl PartialOrd<str> for NodeStr {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other)
    }
}
impl PartialEq<&'_ str> for NodeStr {
    #[inline]
    fn eq(&self, other: &&'_ str) -> bool {
        self.as_str() == *other
    }
}
impl PartialOrd<&'_ str> for NodeStr {
    #[inline]
    fn partial_cmp(&self, other: &&'_ str) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}
impl From<&'_ str> for NodeStr {
    #[inline]
    fn from(value: &'_ str) -> Self {
        Self::new(value)
    }
}
impl From<&'_ String> for NodeStr {
    #[inline]
    fn from(value: &'_ String) -> Self {
        Self::new(value)
    }
}
impl From<String> for NodeStr {
    #[inline]
    fn from(value: String) -> Self {
        Self::Heap(value.into(), None)
    }
}
impl From<&'_ Self> for NodeStr {
    #[inline]
    fn from(value: &'_ Self) -> Self {
        value.clone()
    }
}
impl serde::Serialize for NodeStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> serde::Deserialize<'de> for NodeStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = NodeStr;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.into())
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}
