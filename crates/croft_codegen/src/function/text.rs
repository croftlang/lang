//! String and `Bytes` fields kept in a `croft_text_N` or `croft_bytes_N` of
//! their own size: read where they are, and written with a check, in debug
//! builds, that they fit.

use croft_analysis::types::Resolved;
use croft_analysis::Type;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// Writes `value`, any string, into the field at `place`. Text that
    /// doesn't fit is cut short, and stops a debug build.
    pub fn store_text(&mut self, place: &str, value: &str, capacity: i64, label: &str) {
        let value = self.view_of(value);

        self.open(format!(
            "if (!croft_string_copy({value}, CROFT_OUT({place})))"
        ));

        self.line(format!(
            "CROFT_CHECK(0, \"{label} is longer than {capacity} characters\");"
        ));

        self.close();
    }

    /// Writes `value`, any `Bytes`, into the field at `place`. Values that
    /// don't fit are cut short, and stop a debug build.
    pub fn store_bytes(&mut self, place: &str, value: &str, capacity: i64, label: &str) {
        self.open(format!(
            "if (!croft_bytes_copy(CROFT_BYTES_VIEW({value}), CROFT_BYTES_OUT({place})))"
        ));

        self.line(format!(
            "CROFT_CHECK(0, \"{label} holds at most {capacity} bytes\");"
        ));

        self.close();
    }

    /// Writes `value` into field `field` of `owner` at `place`: by
    /// assignment, or through `store_text` or `store_bytes` when it is kept
    /// in its own size.
    pub fn write_field(&mut self, owner: Option<&Resolved>, place: &str, field: &str, value: &str) {
        if let Some((byte, mask)) = owner.and_then(|owner| self.gen.flag_bit(owner, field)) {
            let flags = format!("{place}.croft_flags[{byte}]");

            self.line(format!(
                "{flags} = (croft_u8) ({value} ? ({flags} | {mask}) : ({flags} & ~{mask}));"
            ));

            return;
        }

        let label = format!("{}: {field}", owner.map_or("", |o| o.name.as_str()));
        let target = format!("{place}.{field}");

        match owner.and_then(|owner| self.gen.own_size(owner, field)) {
            Some(Type::SizedBytes(capacity)) => {
                self.store_bytes(&target, value, i64::from(capacity), &label);
            }
            // A list is already in the field's size: `value` made it so.
            Some(Type::SizedList(..)) => self.line(format!("{target} = {value};")),
            Some(kept) => self.store_text(&target, value, self.gen.bound(&kept), &label),
            None => self.line(format!("{target} = {value};")),
        }
    }

    /// Reads field `field` of `owner` at `place`.
    pub fn read_field(&mut self, owner: Option<&Resolved>, place: &str, field: &str) -> String {
        if let Some((byte, mask)) = owner.and_then(|owner| self.gen.flag_bit(owner, field)) {
            return format!("(({place}.croft_flags[{byte}] & {mask}) != 0)");
        }

        let place = format!("{place}.{field}");

        match owner.and_then(|owner| self.gen.own_size(owner, field)) {
            Some(kept) => self.typed(place, &kept),
            None => place,
        }
    }
}
