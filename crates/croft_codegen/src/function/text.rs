//! String fields kept in a `croft_text_N` of their own size: read into a full
//! string, and written back with a check, in debug builds, that they fit.

use croft_analysis::types::Resolved;
use croft_analysis::Type;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// The field at `place` as a full string.
    pub fn load_text(&mut self, place: &str) -> String {
        let temp = self.temp(&Type::String);

        self.line(format!(
            "croft_text_load({place}.text, {place}.length, &{temp});"
        ));

        temp
    }

    /// Writes `value`, a full string, into the field at `place`. Text that
    /// doesn't fit is cut short, and stops a debug build.
    pub fn store_text(&mut self, place: &str, value: &str, capacity: i64, label: &str) {
        self.open(format!(
            "if (!croft_text_store(&{value}, {place}.text, &{place}.length, {capacity}))"
        ));

        self.line(format!(
            "CROFT_CHECK(0, \"{label} is longer than {capacity} characters\");"
        ));

        self.close();
    }

    /// Writes `value` into field `field` of `owner` at `place`: by
    /// assignment, or through `store_text` when it is kept in its own size.
    pub fn write_field(&mut self, owner: Option<&Resolved>, place: &str, field: &str, value: &str) {
        if let Some((byte, mask)) = owner.and_then(|owner| self.gen.flag_bit(owner, field)) {
            let flags = format!("{place}.croft_flags[{byte}]");

            self.line(format!(
                "{flags} = (croft_u8) ({value} ? ({flags} | {mask}) : ({flags} & ~{mask}));"
            ));

            return;
        }

        match owner.and_then(|owner| self.gen.text_capacity(owner, field)) {
            Some(capacity) => {
                let label = format!("{}: {field}", owner.map_or("", |o| o.name.as_str()));
                self.store_text(&format!("{place}.{field}"), value, capacity, &label);
            }
            None => self.line(format!("{place}.{field} = {value};")),
        }
    }

    /// Reads field `field` of `owner` at `place`.
    pub fn read_field(&mut self, owner: Option<&Resolved>, place: &str, field: &str) -> String {
        if let Some((byte, mask)) = owner.and_then(|owner| self.gen.flag_bit(owner, field)) {
            return format!("(({place}.croft_flags[{byte}] & {mask}) != 0)");
        }

        let place = format!("{place}.{field}");

        match owner.and_then(|owner| self.gen.text_capacity(owner, field)) {
            Some(_) => self.load_text(&place),
            None => place,
        }
    }
}
