pub mod schema;

pub use serde;
pub use serde_json;

use std::{cmp, mem};

use serde::{Deserialize, Deserializer};

use crate::openapi::{Entry, Map};

pub fn deserialize_some<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(T::deserialize(deserializer)?))
}

pub fn merge_maps<K: Ord, V>(
    target: &mut Map<K, V>,
    source: Map<K, V>,
    mut collision: impl FnMut(&mut V, V),
) {
    for (k, v) in source {
        match target.entry(k) {
            Entry::Occupied(mut entry) => {
                collision(entry.get_mut(), v);
            }
            Entry::Vacant(entry) => {
                entry.insert(v);
            }
        }
    }
}

pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

pub fn size_hint<T>(hint: Option<usize>) -> usize {
    const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

    if mem::size_of::<T>() == 0 {
        0
    } else {
        cmp::min(hint.unwrap_or(0), MAX_PREALLOC_BYTES / mem::size_of::<T>())
    }
}
