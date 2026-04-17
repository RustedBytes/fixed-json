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
        // The first `len` entries are initialized exclusively by `attr`.
        let attrs = unsafe {
            slice::from_raw_parts_mut(self.attrs.as_mut_ptr().cast::<Attr<'a>>(), self.len)
        };
        read_object(self.input, attrs)
    }
}

impl<const N: usize> Drop for ObjectBuilder<'_, '_, N> {
    fn drop(&mut self) {
        for attr in &mut self.attrs[..self.len] {
            // Match the initialized prefix maintained by `attr`.
            unsafe {
                ptr::drop_in_place(attr.as_mut_ptr());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectBuilder;
    use crate::{Attr, DefaultValue};

    #[test]
    fn reads_basic_object_with_builder() {
        let mut flag1 = false;
        let mut flag2 = false;
        let mut count = 0;

        let end = ObjectBuilder::<3>::new(r#"{"flag2":false,"count":7,"flag1":true}"#)
            .integer("count", &mut count)
            .boolean("flag1", &mut flag1)
            .boolean("flag2", &mut flag2)
            .read()
            .unwrap();

        assert_eq!(end, 38);
        assert_eq!(count, 7);
        assert!(flag1);
        assert!(!flag2);
    }

    #[test]
    fn accepts_preconfigured_attrs() {
        let mut count = 1;

        ObjectBuilder::<1>::new(r#"{}"#)
            .attr(Attr::integer("count", &mut count).with_default(DefaultValue::Integer(5)))
            .read()
            .unwrap();

        assert_eq!(count, 5);
    }
}
