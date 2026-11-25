//! How long a string can be. Code generation works out a bound for each
//! string it builds, from literals, `Length` constraints and the methods that
//! make one string from another, and keeps it in a `croft_text_N` of just
//! that size. A string with no bound below the shared size is a full
//! `croft_string`.

use croft_analysis::Type;

use crate::Generator;

/// Whether a value of `ty` is a string, of any size.
pub fn is_string(ty: &Type) -> bool {
    matches!(ty, Type::String | Type::Text(_) | Type::Borrowed(_))
}

/// `ty` with every bounded string a plain `String` again: the type the
/// checker gave it.
pub fn unbounded(ty: &Type) -> Type {
    match ty {
        Type::Text(_) | Type::Borrowed(_) => Type::String,
        Type::List(item) => Type::list(unbounded(item)),
        Type::Option(item) => Type::option(unbounded(item)),
        Type::Map(key, value) => Type::map(unbounded(key), unbounded(value)),
        Type::Result(value, error) => Type::result(unbounded(value), unbounded(error)),
        other => other.clone(),
    }
}

/// Whether `ty` holds a bounded string anywhere.
pub fn has_bound(ty: &Type) -> bool {
    match ty {
        Type::Text(_) | Type::Borrowed(_) => true,
        Type::List(item) | Type::Option(item) => has_bound(item),
        Type::Map(key, value) | Type::Result(key, value) => has_bound(key) || has_bound(value),
        _ => false,
    }
}

impl<'a> Generator<'a> {
    /// A string of at most `length` characters: bounded when that is less
    /// than the shared size. C has no empty arrays, so it holds at least one.
    pub fn text_type(&self, length: i64) -> Type {
        if length >= self.sizes.string_capacity {
            return Type::String;
        }

        Type::Text(length.max(1) as u16)
    }

    /// `ty` with each string as short as it can be: the type of `None`,
    /// `[]` or `Err(e)`, which hold no string at all.
    pub fn narrowest(&self, ty: &Type) -> Type {
        match ty {
            Type::String | Type::Text(_) => self.text_type(1),
            Type::List(item) => Type::list(self.narrowest(item)),
            Type::Option(item) => Type::option(self.narrowest(item)),
            Type::Map(key, value) => Type::map(self.narrowest(key), self.narrowest(value)),
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
            Type::List(item) => Type::list(self.owned_type(item)),
            Type::Option(item) => Type::option(self.owned_type(item)),
            Type::Map(key, value) => Type::map(self.owned_type(key), self.owned_type(value)),
            Type::Result(value, error) => {
                Type::result(self.owned_type(value), self.owned_type(error))
            }
            other => other.clone(),
        }
    }

    /// The most characters a string of `ty` holds.
    pub fn bound(&self, ty: &Type) -> i64 {
        match ty {
            Type::Text(length) | Type::Borrowed(length) => i64::from(*length),
            _ => self.sizes.string_capacity,
        }
    }

    /// A type with room for a value of either `a` or `b`, which differ only
    /// in the bounds of their strings.
    /// Two borrowed strings give a borrowed string; a borrowed string and
    /// one of its own give one of its own.
    pub fn wider(&self, a: &Type, b: &Type) -> Type {
        match (a, b) {
            (Type::Borrowed(a), Type::Borrowed(b)) => Type::Borrowed((*a).max(*b)),
            (a, b) if is_string(a) && is_string(b) => {
                self.text_type(self.bound(a).max(self.bound(b)))
            }
            (Type::List(a), Type::List(b)) => Type::list(self.wider(a, b)),
            (Type::Option(a), Type::Option(b)) => Type::option(self.wider(a, b)),
            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                Type::map(self.wider(ak, bk), self.wider(av, bv))
            }
            (Type::Result(av, ae), Type::Result(bv, be)) => {
                Type::result(self.wider(av, bv), self.wider(ae, be))
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
