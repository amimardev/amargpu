/// A type-erased, artificially-'static wrapper around a borrowed reference.
/// SAFETY INVARIANT: the value must be removed from the registry before
/// the real borrow (`'a`) ends. Never let this outlive the reference it wraps.
pub struct StaticRef<T: ?Sized>(*const T);

// Only sound if you're single-threaded, or you're certain T's usage across
// threads is fine — ActiveEventLoop is !Send/!Sync anyway typically, so be careful.
unsafe impl<T: ?Sized> Send for StaticRef<T> {}
unsafe impl<T: ?Sized> Sync for StaticRef<T> {}

impl<T: ?Sized> StaticRef<T> {
    /// SAFETY: caller guarantees this is removed from the registry
    /// before `value` (or its referent) is dropped / goes out of scope.
    pub unsafe fn new(value: &T) -> Self {
        StaticRef(value as *const T)
    }

    pub fn get(&self) -> &T {
        // SAFETY: relies on the caller of `new` upholding the invariant above.
        unsafe { &*self.0 }
    }
}

/// A type-erased, artificially-'static wrapper around a borrowed reference.
/// SAFETY INVARIANT: the value must be removed from the registry before
/// the real borrow (`'a`) ends. Never let this outlive the reference it wraps.
pub struct StaticMut<T: ?Sized>(*mut T);

// Only sound if you're single-threaded, or you're certain T's usage across
// threads is fine — ActiveEventLoop is !Send/!Sync anyway typically, so be careful.
unsafe impl<T: ?Sized> Send for StaticMut<T> {}
unsafe impl<T: ?Sized> Sync for StaticMut<T> {}

impl<T: ?Sized> StaticMut<T> {
    /// SAFETY: caller guarantees this is removed from the registry
    /// before `value` (or its referent) is dropped / goes out of scope.
    pub unsafe fn new(value: &mut T) -> Self {
        StaticMut(value as *mut T)
    }

    pub fn get(&self) -> &T {
        // SAFETY: relies on the caller of `new` upholding the invariant above.
        unsafe { &*self.0 }
    }
}