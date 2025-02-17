use core::{fmt::Debug, marker::PhantomData};

/// A value containing an allocation living for `'a`
pub struct Allocated<'a, C> {
    inner: C,
    _phantom: PhantomData<&'a mut C>,
}

impl<'a, C> Allocated<'a, C> {
    pub(crate) fn new(inner: C) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<C> core::ops::Deref for Allocated<'_, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<C> core::ops::DerefMut for Allocated<'_, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(PartialEq, Eq)]
pub struct Allocation<T> {
    pub(crate) ptr: *mut T,
    pub(crate) len: usize,
}

impl<T> Allocation<T> {
    pub const fn new() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            len: 0,
        }
    }

    pub fn as_slice(&self) -> &[T] {
        if self.ptr.is_null() {
            panic!("Allocation is empty");
        }

        // Safety: The pointer is valid for the entire live of this allocation
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        if self.ptr.is_null() {
            panic!("Allocation is empty");
        }

        // Safety: The pointer is valid for the entire live of this allocation
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> Default for Allocation<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug> Debug for Allocation<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.ptr.is_null() {
            let empty: &[T] = &[];
            f.debug_struct("Allocation").field("value", &empty).finish()
        } else {
            f.debug_struct("Allocation")
                .field("value", &self.as_slice())
                .finish()
        }
    }
}

unsafe impl<T: Send> Send for Allocation<T> {}
unsafe impl<T: Sync> Sync for Allocation<T> {}
