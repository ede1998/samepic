use std::cmp::Eq;
use std::hash::Hash;
use std::marker::PhantomData;

type LruLruCache<K, V> = lru::LruCache<K, V>;

pub struct LruCache<T> {
    cache: LruLruCache<Key<T>, T>,
    current_key: Key<T>,
}

impl<T> LruCache<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruLruCache::new(capacity),
            current_key: Key::new(),
        }
    }

    pub fn push(&mut self, value: T) -> Key<T> {
        let key = self.current_key;
        self.cache.push(key, value);
        self.current_key = key.next();
        key
    }

    pub fn get_or_insert<F>(&mut self, k: Key<T>, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        self.cache.get_or_insert(k, f).expect("zero sized cache")
    }
}

pub struct Key<T>(u32, PhantomData<T>);

impl<T> Copy for Key<T> {}

impl<T> std::fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Key<{}>({:?})", std::any::type_name::<T>(), self.0)
    }
}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T> Eq for Key<T> {}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Hash for Key<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> Key<T> {
    fn new() -> Self {
        Self(0, PhantomData::default())
    }
    fn next(self) -> Self {
        Self(self.0 + 1, PhantomData::default())
    }
}
