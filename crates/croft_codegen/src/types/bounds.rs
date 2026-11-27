//! How long a string, a `Bytes` or a list can be. Code generation works out a
//! bound for each one it builds, from literals, `Length` constraints and the
//! methods that make one from another, and keeps it in a `croft_text_N`, a
//! `croft_bytes_N` or a `croft_listN_...` of just that size. A string or
//! `Bytes` with no bound is a full `croft_string` or `croft_bytes`, of the
//! target's `strings_length` or `bytes_length`, and a list with no bound
//! below the shared size a full `croft_list_...`.

use croft_analysis::Type;

use crate::Generator;

/// Whether a value of `ty` is a string, of any size.
pub fn is_string(ty: &Type) -> bool {
    matches!(ty, Type::String | Type::Text(_) | Type::Borrowed(_))
}

/// Whether a value of `ty` is `Bytes`, of any size.
pub fn is_bytes(ty: &Type) -> bool {
    matches!(ty, Type::Bytes | Type::SizedBytes(_))
}

/// The type of a list's items, whatever its size.
pub fn list_item(ty: &Type) -> Option<&Type> {
    match ty {
        Type::List(item) | Type::SizedList(item, _) => Some(item),
        _ => None,
    }
}

/// `ty` with every bounded string a plain `String` again, every bounded
/// `Bytes` a plain `Bytes` and every sized list a plain `List`: the type the
/// checker gave it.
pub fn unbounded(ty: &Type) -> Type {
    match ty {
        Type::Text(_) | Type::Borrowed(_) => Type::String,
        Type::SizedBytes(_) => Type::Bytes,
        Type::List(item) | Type::SizedList(item, _) => Type::list(unbounded(item)),
        Type::Option(item) => Type::option(unbounded(item)),
        Type::Map(key, value) => Type::map(unbounded(key), unbounded(value)),
        Type::Set(item) => Type::set(unbounded(item)),
        Type::Result(value, error) => Type::result(unbounded(value), unbounded(error)),
        other => other.clone(),
    }
}

/// Whether `ty` holds a bounded string, `Bytes` or list anywhere.
pub fn has_bound(ty: &Type) -> bool {
    match ty {
        Type::Text(_) | Type::Borrowed(_) | Type::SizedBytes(_) | Type::SizedList(..) => true,
        Type::List(item) | Type::Set(item) | Type::Option(item) => has_bound(item),
        Type::Map(key, value) | Type::Result(key, value) => has_bound(key) || has_bound(value),
        _ => false,
    }
}

impl<'a> Generator<'a> {
    /// A string of at most `length` characters: a full string when that is
    /// the size a string with no `Length` has, and a size of its own, shorter
    /// or longer, otherwise. None is kept in more than the most any string
    /// can need, and as C has no empty arrays, each holds at least one.
    pub fn text_type(&self, length: i64) -> Type {
        let length = length.min(self.sizes.string_largest).max(1);

        if length == self.sizes.string_capacity {
            return Type::String;
        }

        Type::Text(length as u16)
    }

    /// `Bytes` of at most `length` values: a full `Bytes` when that is the
    /// size a `Bytes` with no `Length` has, and a size of its own, shorter or
    /// longer, otherwise. None is kept in more than the most any `Bytes` can
    /// need, and like a string, each holds at least one.
    pub fn bytes_type(&self, length: i64) -> Type {
        let length = length.min(self.sizes.bytes_largest).max(1);

        if length == self.sizes.bytes_capacity {
            return Type::Bytes;
        }

        Type::SizedBytes(length as u16)
    }

    /// A list of at most `length` items: sized when that is less than the
    /// shared size for lists of `item`. Like a string, it holds at least one.
    pub fn list_type(&self, item: Type, length: i64) -> Type {
        if length >= self.sizes.list_capacity(&item) {
            return Type::list(item);
        }

        Type::SizedList(Box::new(item), length.max(1) as u16)
    }

    /// The most items a list of `ty` holds.
    pub fn capacity(&self, ty: &Type) -> i64 {
        match ty {
            Type::SizedList(_, length) => i64::from(*length),
            Type::List(item) => self.sizes.list_capacity(item),
            _ => 0,
        }
    }

    /// A list of `ty`'s size holding items of `item`.
    pub fn with_item(&self, ty: &Type, item: Type) -> Type {
        self.list_type(item, self.capacity(ty))
    }

    /// `ty` with each string as short as it can be and each list as small:
    /// the type of `None`, `[]` or `Err(e)`, which hold nothing at all.
    pub fn narrowest(&self, ty: &Type) -> Type {
        match ty {
            Type::String | Type::Text(_) => self.text_type(1),
            Type::Bytes | Type::SizedBytes(_) => self.bytes_type(1),
            Type::List(item) | Type::SizedList(item, _) => self.list_type(self.narrowest(item), 1),
            Type::Option(item) => Type::option(self.narrowest(item)),
            Type::Map(key, value) => Type::map(self.narrowest(key), self.narrowest(value)),
            Type::Set(item) => Type::set(self.narrowest(item)),
            Type::Result(value, error) => {
                Type::result(self.narrowest(value), self.narrowest(error))
            }
            other => other.clone(),
        }
    }

    /// `ty` with each borrowed string a string of its own, as a value that
    /// leaves the function, or is written to, needs.
    pub fn owned_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Borrowed(length) => self.text_type(i64::from(*length)),
            Type::List(item) | Type::SizedList(item, _) => {
                self.with_item(ty, self.owned_type(item))
            }
            Type::Option(item) => Type::option(self.owned_type(item)),
            Type::Map(key, value) => Type::map(self.owned_type(key), self.owned_type(value)),
            Type::Set(item) => Type::set(self.owned_type(item)),
            Type::Result(value, error) => {
                Type::result(self.owned_type(value), self.owned_type(error))
            }
            other => other.clone(),
        }
    }

    /// The most characters a string of `ty` holds, or values a `Bytes` does.
    pub fn bound(&self, ty: &Type) -> i64 {
        match ty {
            Type::Text(length) | Type::Borrowed(length) | Type::SizedBytes(length) => {
                i64::from(*length)
            }
            Type::Bytes => self.sizes.bytes_capacity,
            _ => self.sizes.string_capacity,
        }
    }

    /// A type with room for a value of either `a` or `b`, which differ only
    /// in the bounds of their strings, `Bytes` and lists.
    /// Two borrowed strings give a borrowed string; a borrowed string and
    /// one of its own give one of its own.
    pub fn wider(&self, a: &Type, b: &Type) -> Type {
        if let (Some(a_item), Some(b_item)) = (list_item(a), list_item(b)) {
            let length = self.capacity(a).max(self.capacity(b));
            return self.list_type(self.wider(a_item, b_item), length);
        }

        match (a, b) {
            (Type::Borrowed(a), Type::Borrowed(b)) => Type::Borrowed((*a).max(*b)),
            (a, b) if is_string(a) && is_string(b) => {
                self.text_type(self.bound(a).max(self.bound(b)))
            }
            (a, b) if is_bytes(a) && is_bytes(b) => {
                self.bytes_type(self.bound(a).max(self.bound(b)))
            }
            (Type::Option(a), Type::Option(b)) => Type::option(self.wider(a, b)),
            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                Type::map(self.wider(ak, bk), self.wider(av, bv))
            }
            (Type::Set(a), Type::Set(b)) => Type::set(self.wider(a, b)),
            (Type::Result(av, ae), Type::Result(bv, be)) => {
                Type::result(self.wider(av, bv), self.wider(ae, be))
            }
            (a, _) => a.clone(),
        }
    }

    /// A type with room for no more than either `a` or `b` holds, which
    /// differ only in the bounds of their strings, `Bytes` and lists: what a
    /// function gives back, kept within its declared result.
    pub fn narrower(&self, a: &Type, b: &Type) -> Type {
        if let (Some(a_item), Some(b_item)) = (list_item(a), list_item(b)) {
            let length = self.capacity(a).min(self.capacity(b));
            return self.list_type(self.narrower(a_item, b_item), length);
        }

        match (a, b) {
            (a, b) if is_string(a) && is_string(b) => {
                self.text_type(self.bound(a).min(self.bound(b)))
            }
            (a, b) if is_bytes(a) && is_bytes(b) => {
                self.bytes_type(self.bound(a).min(self.bound(b)))
            }
            (Type::Option(a), Type::Option(b)) => Type::option(self.narrower(a, b)),
            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                Type::map(self.narrower(ak, bk), self.narrower(av, bv))
            }
            (Type::Result(av, ae), Type::Result(bv, be)) => {
                Type::result(self.narrower(av, bv), self.narrower(ae, be))
            }
            (a, _) => a.clone(),
        }
    }

    /// `bounded` when it is `ty` with only its strings' bounds worked out;
    /// otherwise `ty`, as when a value is passed where a wider type is wanted.
    pub fn keep_bound(&self, bounded: &Type, ty: &Type) -> Type {
        if unbounded(bounded) == unbounded(ty) {
            return bounded.clone();
        }

        ty.clone()
    }
}
