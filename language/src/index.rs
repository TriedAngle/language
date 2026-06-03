use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

pub trait Idx: Copy + Eq + Hash + Debug {
    fn from_usize(i: usize) -> Self;
    fn index(self) -> usize;
}

#[macro_export]
macro_rules! make_index {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name(u32);

        impl $crate::index::Idx for $name {
            #[inline]
            fn from_usize(i: usize) -> Self {
                debug_assert!(i <= u32::MAX as usize, "index overflow");
                $name(i as u32)
            }
            #[inline]
            fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct IndexVec<I: Idx, V> {
    raw: Vec<V>,
    _marker: PhantomData<fn() -> I>,
}

impl<I: Idx, V> IndexVec<I, V> {
    pub fn new() -> Self {
        Self {
            raw: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn push(&mut self, v: V) -> I {
        let i = I::from_usize(self.raw.len());
        self.raw.push(v);
        i
    }

    pub fn get(&self, i: I) -> Option<&V> {
        self.raw.get(i.index())
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn next_idx(&self) -> I {
        I::from_usize(self.raw.len())
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.raw.iter()
    }

    pub fn iter_enumerated(&self) -> impl Iterator<Item = (I, &V)> {
        self.raw
            .iter()
            .enumerate()
            .map(|(i, v)| (I::from_usize(i), v))
    }
}

impl<I: Idx, V> Default for IndexVec<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: Idx, V> Index<I> for IndexVec<I, V> {
    type Output = V;
    fn index(&self, i: I) -> &V {
        &self.raw[i.index()]
    }
}

impl<I: Idx, V> IndexMut<I> for IndexVec<I, V> {
    fn index_mut(&mut self, i: I) -> &mut V {
        &mut self.raw[i.index()]
    }
}

impl<I: Idx, V> Index<Range<I>> for IndexVec<I, V> {
    type Output = [V];
    fn index(&self, index: Range<I>) -> &Self::Output {
        &self.raw[index.start.index()..index.end.index()]
    }
}

impl<I: Idx, V> IndexMut<Range<I>> for IndexVec<I, V> {
    fn index_mut(&mut self, index: Range<I>) -> &mut Self::Output {
        &mut self.raw[index.start.index()..index.end.index()]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RawRange<T> {
    start: T,
    end: T,
}

impl<I: Idx> From<RawRange<I>> for Range<I> {
    fn from(value: RawRange<I>) -> Self {
        Range {
            start: value.start,
            end: value.end,
        }
    }
}

impl<I: Idx> From<Range<I>> for RawRange<I> {
    fn from(value: Range<I>) -> Self {
        RawRange {
            start: value.start,
            end: value.end,
        }
    }
}

impl<I: Idx, V> Index<RawRange<I>> for IndexVec<I, V> {
    type Output = [V];
    fn index(&self, index: RawRange<I>) -> &Self::Output {
        &self.raw[index.start.index()..index.end.index()]
    }
}

impl<I: Idx, V> IndexMut<RawRange<I>> for IndexVec<I, V> {
    fn index_mut(&mut self, index: RawRange<I>) -> &mut Self::Output {
        &mut self.raw[index.start.index()..index.end.index()]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct IdRange<I> {
    start: u32,
    len: u32,
    _marker: PhantomData<I>,
}

impl<I> IdRange<I> {
    pub const fn empty() -> Self {
        Self {
            start: 0,
            len: 0,
            _marker: PhantomData,
        }
    }
    pub fn len(self) -> usize {
        self.len as usize
    }
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

pub struct IndexPool<I: Idx, V> {
    items: IndexVec<I, V>,
    pool: Vec<I>,
}

impl<I: Idx, V> IndexPool<I, V> {
    pub fn new() -> Self {
        Self {
            items: IndexVec::new(),
            pool: Vec::new(),
        }
    }

    pub fn alloc(&mut self, v: V) -> I {
        self.items.push(v)
    }
    pub fn get(&self, i: I) -> &V {
        &self.items[i]
    }
    pub fn get_mut(&mut self, i: I) -> &mut V {
        &mut self.items[i]
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn next_idx(&self) -> I {
        self.items.next_idx()
    }
    pub fn iter_enumerated(&self) -> impl Iterator<Item = (I, &V)> {
        self.items.iter_enumerated()
    }

    pub fn alloc_range(&mut self, ids: &[I]) -> IdRange<I> {
        let start = self.pool.len() as u32;
        self.pool.extend_from_slice(ids);
        IdRange {
            start,
            len: ids.len() as u32,
            _marker: PhantomData,
        }
    }

    pub fn range(&self, r: IdRange<I>) -> &[I] {
        let s = r.start as usize;
        &self.pool[s..s + r.len as usize]
    }
}

impl<I: Idx, V> Default for IndexPool<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: Idx, V> Index<I> for IndexPool<I, V> {
    type Output = V;
    fn index(&self, i: I) -> &V {
        &self.items[i]
    }
}

impl<I: Idx, V> IndexMut<I> for IndexPool<I, V> {
    fn index_mut(&mut self, i: I) -> &mut V {
        &mut self.items[i]
    }
}
