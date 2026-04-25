use core::{array, mem::MaybeUninit, ptr, slice};

use crate::{Array, Attr, Result, read_object};

pub struct ObjectBuilder<'input, 'a, const N: usize> {
    input: &'input str,
    attrs: [MaybeUninit<Attr<'a>>; N],
    len: usize,
}

impl<'input, 'a, const N: usize> ObjectBuilder<'input, 'a, N> {
    #[inline]
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            attrs: array::from_fn(|_| MaybeUninit::uninit()),
            len: 0,
        }
    }

    #[inline]
    pub fn attr(mut self, attr: Attr<'a>) -> Self {
        assert!(self.len < N, "too many attributes for ObjectBuilder");
        self.attrs[self.len].write(attr);
        self.len += 1;
        self
    }

    #[inline]
    pub fn integer(self, name: &'a str, target: &'a mut i32) -> Self {
        self.attr(Attr::integer(name, target))
    }

    #[inline]
    pub fn integers(self, name: &'a str, target: &'a mut [i32]) -> Self {
        self.attr(Attr::integers(name, target))
    }

    #[inline]
    pub fn uinteger(self, name: &'a str, target: &'a mut u32) -> Self {
        self.attr(Attr::uinteger(name, target))
    }

    #[inline]
    pub fn uintegers(self, name: &'a str, target: &'a mut [u32]) -> Self {
        self.attr(Attr::uintegers(name, target))
    }

    #[inline]
    pub fn short(self, name: &'a str, target: &'a mut i16) -> Self {
        self.attr(Attr::short(name, target))
    }

    #[inline]
    pub fn shorts(self, name: &'a str, target: &'a mut [i16]) -> Self {
        self.attr(Attr::shorts(name, target))
    }

    #[inline]
    pub fn ushort(self, name: &'a str, target: &'a mut u16) -> Self {
        self.attr(Attr::ushort(name, target))
    }

    #[inline]
    pub fn ushorts(self, name: &'a str, target: &'a mut [u16]) -> Self {
        self.attr(Attr::ushorts(name, target))
    }
}

impl<'input, 'a, const N: usize> ObjectBuilder<'input, 'a, N> {
    #[inline]
    pub fn real(self, name: &'a str, target: &'a mut f64) -> Self {
        self.attr(Attr::real(name, target))
    }

    #[inline]
    pub fn reals(self, name: &'a str, target: &'a mut [f64]) -> Self {
        self.attr(Attr::reals(name, target))
    }

    #[inline]
    pub fn string(self, name: &'a str, target: &'a mut [u8]) -> Self {
        self.attr(Attr::string(name, target))
    }

    #[inline]
    pub fn boolean(self, name: &'a str, target: &'a mut bool) -> Self {
        self.attr(Attr::boolean(name, target))
    }

    #[inline]
    pub fn booleans(self, name: &'a str, target: &'a mut [bool]) -> Self {
        self.attr(Attr::booleans(name, target))
    }

    #[inline]
    pub fn character(self, name: &'a str, target: &'a mut u8) -> Self {
        self.attr(Attr::character(name, target))
    }

    #[inline]
    pub fn characters(self, name: &'a str, target: &'a mut [u8]) -> Self {
        self.attr(Attr::characters(name, target))
    }
}

impl<'input, 'a, const N: usize> ObjectBuilder<'input, 'a, N> {
    #[inline]
    pub fn time(self, name: &'a str, target: &'a mut f64) -> Self {
        self.attr(Attr::time(name, target))
    }

    #[inline]
    pub fn object(self, name: &'a str, attrs: &'a mut [Attr<'a>]) -> Self {
        self.attr(Attr::object(name, attrs))
    }

    #[inline]
    pub fn array(self, name: &'a str, array: Array<'a>) -> Self {
        self.attr(Attr::array(name, array))
    }

    #[inline]
    pub fn check(self, name: &'a str, expected: &'a str) -> Self {
        self.attr(Attr::check(name, expected))
    }

    #[inline]
    pub fn ignore_any(self) -> Self {
        self.attr(Attr::ignore_any())
    }

    #[inline]
    pub fn read(mut self) -> Result<usize> {
        // SAFETY: `self.attrs` is initialized up to `self.len` by the builder methods, and we only read that prefix.
        let attrs = unsafe {
            slice::from_raw_parts_mut(self.attrs.as_mut_ptr().cast::<Attr<'a>>(), self.len)
        };
        read_object(self.input, attrs)
    }
}

impl<const N: usize> Drop for ObjectBuilder<'_, '_, N> {
    fn drop(&mut self) {
        for attr in &mut self.attrs[..self.len] {
            // SAFETY: `self.attrs` is initialized up to `self.len` by the builder methods, and we only drop that prefix.
            unsafe {
                ptr::drop_in_place(attr.as_mut_ptr());
            }
        }
    }
}
