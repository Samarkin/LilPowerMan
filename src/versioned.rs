use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

/// Provides a wrapper around `T` that speeds up comparison at the cost of extra memory.
pub struct Versioned<T: PartialEq> {
    inner: T,
    version: u64,
}

impl<T: PartialEq> Versioned<T> {
    pub fn new(inner: T) -> Self {
        Versioned {
            inner,
            version: fastrand::u64(..),
        }
    }

    fn bump_version(&mut self) {
        let candidate = fastrand::u64(..);
        self.version = if candidate == self.version {
            // ensure the new version is different even in case of a collision
            candidate + 1
        } else {
            candidate
        }
    }
}

impl<T: PartialEq + Clone> Clone for Versioned<T> {
    fn clone(&self) -> Self {
        Versioned {
            inner: self.inner.clone(),
            version: self.version,
        }
    }
}

impl<T: PartialEq> Deref for Versioned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: PartialEq> DerefMut for Versioned<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.bump_version();
        &mut self.inner
    }
}

impl<T: PartialEq> PartialEq for Versioned<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.version == other.version {
            return true;
        }
        self.inner == other.inner
    }
}

impl<T: PartialEq + Eq> Eq for Versioned<T> {}

impl<T: PartialEq + Debug> Debug for Versioned<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test() {
        let mut a = Versioned::new(vec![]);
        let mut b = a.clone();
        assert_eq!(a, b);

        _ = a.deref_mut();
        assert_eq!(a, b);

        a.push(10);
        assert_ne!(a, b);

        b.push(10);
        assert_eq!(a, b);
    }

    #[test]
    fn test_laziness() {
        #[derive(Clone, Debug)]
        struct A {
            eq_d: RefCell<bool>,
        }
        impl PartialEq for A {
            fn eq(&self, _other: &Self) -> bool {
                self.eq_d.replace(true);
                true
            }
        }

        let mut a = Versioned::new(A {
            eq_d: RefCell::new(false),
        });
        let b = a.clone();
        _ = a == b;
        assert_eq!(a.eq_d.take(), false);
        assert_eq!(b.eq_d.take(), false);

        _ = a.deref();
        _ = a == b;
        assert_eq!(a.eq_d.take(), false);
        assert_eq!(b.eq_d.take(), false);

        _ = a.deref_mut();
        _ = a == b;
        assert_eq!(b.eq_d.take(), false);
        assert_eq!(a.eq_d.take(), true);

        _ = b == a;
        assert_eq!(b.eq_d.take(), true);
    }
}
