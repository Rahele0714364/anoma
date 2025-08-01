//! The storage prefix iterators can be used to iterate over a common prefix of
//! storage keys.

use std::collections::HashMap;

use crate::ledger::storage;

/// A temporary iterators storage, used during a wasm run after which it's
/// dropped. Each iterator is assigned a [`PrefixIteratorId`].
pub struct PrefixIterators<'iter, DB>
where
    DB: storage::DBIter<'iter>,
{
    index: PrefixIteratorId,
    iterators: HashMap<PrefixIteratorId, DB::PrefixIter>,
}

impl<'iter, DB> PrefixIterators<'iter, DB>
where
    DB: storage::DBIter<'iter>,
{
    /// Insert a new prefix iterator to the temporary storage.
    pub fn insert(&mut self, iter: DB::PrefixIter) -> PrefixIteratorId {
        let id = self.index;
        self.iterators.insert(id, iter);
        self.index = id.next_id();
        id
    }

    /// Get the next item in the given prefix iterator.
    pub fn next(
        &mut self,
        id: PrefixIteratorId,
    ) -> Option<<DB::PrefixIter as Iterator>::Item> {
        match self.iterators.get_mut(&id) {
            Some(iter) => iter.next(),
            None => None,
        }
    }
}

impl<'iter, DB> Default for PrefixIterators<'iter, DB>
where
    DB: storage::DBIter<'iter>,
{
    fn default() -> Self {
        Self {
            index: PrefixIteratorId::new(0),
            iterators: HashMap::new(),
        }
    }
}

/// A prefix iterator identifier for the temporary storage [`PrefixIterators`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PrefixIteratorId(u64);

impl PrefixIteratorId {
    /// Initialize a new ID.
    pub fn new(id: u64) -> Self {
        PrefixIteratorId(id)
    }

    /// Get the ID as `u64`.
    pub fn id(&self) -> u64 {
        self.0
    }

    /// Get the ID for the next prefix iterator.
    fn next_id(&self) -> PrefixIteratorId {
        PrefixIteratorId(self.0 + 1)
    }
}
