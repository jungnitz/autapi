use std::{borrow::Cow, cell::Cell, collections::BTreeMap, iter::Map, marker::PhantomData, slice};

use serde::ser::{Error, SerializeSeq};

use crate::{
    Registry,
    adapters::SerdeAdapter,
    openapi::{MaybeRef, Schema},
    schema::{SchemaSerialize, ToSchema},
};

/// Implements [`SchemaSerialize`] for an iterator by serializing the elements into a sequence.
///
/// A value of this type should only be serialized one.
/// Serializing multiple times will result in an error.
pub struct IteratorArray<'i, I, M = ()>(Cell<Option<I>>, M, PhantomData<&'i ()>);

pub type BoxedIteratorArray<'i, T> = IteratorArray<'i, Box<dyn Iterator<Item = T> + 'i>>;

pub type SliceIteratorArray<'a, T> = IteratorArray<'a, slice::Iter<'a, T>>;

pub type MappedSliceIteratorArray<'a, Mapped, T> =
    IteratorArray<'a, Map<slice::Iter<'a, T>, fn(&'a T) -> Mapped>>;

impl<I> IteratorArray<'_, I> {
    pub fn new(iter: I) -> Self {
        Self::with_mapper(iter, ())
    }
}

impl<I, M> IteratorArray<'_, I, M> {
    pub fn with_mapper(iter: I, mapper: M) -> Self {
        Self(Cell::new(Some(iter)), mapper, Default::default())
    }
}

impl<'i, T> BoxedIteratorArray<'i, T> {
    pub fn boxed(iter: impl IntoIterator<Item = T> + 'i) -> Self {
        Self::new(Box::new(iter.into_iter()))
    }
}

impl<'a, Mapped, T> MappedSliceIteratorArray<'a, Mapped, T> {
    pub fn map_slice(slice: &'a [T], map: fn(&'a T) -> Mapped) -> Self {
        Self::new(slice.iter().map(map))
    }
}

impl<'a, T> SliceIteratorArray<'a, T> {
    pub fn slice(slice: &'a [T]) -> Self {
        Self::new(slice.iter())
    }
}

impl<'i, I, M> ToSchema for IteratorArray<'i, I, M>
where
    I: Iterator,
    M: IteratorItemMapper<I::Item>,
{
    type Original = <[M::Original] as ToSchema>::Original;

    const REQUIRED: bool = true;
    const ALWAYS_INLINED: bool = <[M::Original] as ToSchema>::ALWAYS_INLINED;

    fn name() -> Cow<'static, str> {
        <[M::Original] as ToSchema>::name()
    }

    fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
        <[M::Original] as ToSchema>::schema(registry)
    }
}

impl<'i, I, M> SchemaSerialize for IteratorArray<'i, I, M>
where
    I: Iterator + 'i,
    M: IteratorItemMapper<I::Item>,
{
    fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let iterator = self
            .0
            .take()
            .ok_or(S::Error::custom("IteratorArray serialized twice"))?;
        let mut seq = serializer.serialize_seq(None)?;
        for item in iterator {
            let item = self.1.map(&item);
            let adapter = SerdeAdapter(item);
            if !adapter.skip_serializing() {
                seq.serialize_element(&adapter)?;
            }
        }
        seq.end()
    }

    fn is_present(&self) -> bool {
        true
    }
}

pub trait IteratorItemMapper<Item> {
    type Output<'i>: ToSchema<Original = Self::Original> + SchemaSerialize
    where
        Item: 'i;
    type Original: ToSchema;
    fn map<'i>(&self, value: &'i Item) -> Self::Output<'i>;
}

impl<I> IteratorItemMapper<I> for ()
where
    I: ToSchema + SchemaSerialize,
{
    type Output<'i>
        = &'i I
    where
        I: 'i;
    type Original = I::Original;

    fn map<'i>(&self, value: &'i I) -> Self::Output<'i> {
        value
    }
}

/// Implements [`SchemaSerialize`] for an iterator by serializing the key/value-pairs into a map.
///
/// A value of this type should only be serialized one.
/// Serializing multiple times will result in an error.
pub struct IteratorMap<I>(Cell<Option<I>>);

pub type BoxedIteratorMap<'i, K, V> = IteratorMap<Box<dyn Iterator<Item = (K, V)> + 'i>>;

impl<I> IteratorMap<I> {
    pub fn new(iter: I) -> Self {
        Self(Cell::new(Some(iter)))
    }
}

impl<'i, K, V> BoxedIteratorMap<'i, K, V> {
    pub fn boxed(iter: impl IntoIterator<Item = (K, V)> + 'i) -> Self {
        Self::new(Box::new(iter.into_iter()))
    }
}

impl<I, K, V> ToSchema for IteratorMap<I>
where
    I: Iterator<Item = (K, V)>,
    K: ToSchema,
    V: ToSchema,
{
    type Original = <BTreeMap<K, V> as ToSchema>::Original;

    const REQUIRED: bool = true;
    const ALWAYS_INLINED: bool = <BTreeMap<K, V> as ToSchema>::ALWAYS_INLINED;

    fn name() -> Cow<'static, str> {
        <BTreeMap<K, V> as ToSchema>::name()
    }

    fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
        <BTreeMap<K, V> as ToSchema>::schema(registry)
    }
}

impl<I, K, V> SchemaSerialize for IteratorMap<I>
where
    I: Iterator<Item = (K, V)>,
    K: SchemaSerialize,
    V: SchemaSerialize,
{
    fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(
            self.0
                .take()
                .ok_or_else(|| S::Error::custom("IteratorMap serialized twice"))?
                .map(|(k, v)| (SerdeAdapter(k), SerdeAdapter(v))),
        )
    }

    fn is_present(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;

    use super::*;

    #[test]
    pub fn array() {
        let mut iter = [1, 2, 3].into_iter();
        assert_json_snapshot!(SerdeAdapter(IteratorArray::new(&mut iter)));
    }
    #[test]
    pub fn map() {
        let mut iter = [("a", 1), ("b", 2)].into_iter();
        assert_json_snapshot!(SerdeAdapter(IteratorMap::new(&mut iter)));
    }
}
