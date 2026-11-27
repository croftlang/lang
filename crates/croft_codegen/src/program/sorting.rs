//! Putting a list in order, written as functions on demand: a stable merge
//! sort that moves the items' places rather than the items themselves.

use croft_analysis::Type;

use crate::types::bounds::is_string;
use crate::Generator;

impl<'a> Generator<'a> {
    /// The name of the function that puts the places of `length` items of
    /// `item`, a number or a string, in order, generated later. Every list
    /// whose items have the same C type shares one.
    ///
    /// It is called as `name(items, length, order, room)`: `order` comes
    /// back holding each item's place, first to last, and `room` is as long
    /// and only used while it works.
    pub fn sort_order_for(&self, item: &Type) -> String {
        let c_type = self.c_type(item);
        let name = format!("croft_order_{}", c_type.trim_start_matches("croft_"));

        let mut sorts = self.sorts.borrow_mut();

        if sorts.iter().any(|(known, _, _)| *known == name) {
            return name;
        }

        let comes_first = if is_string(item) {
            "croft_string_compare(CROFT_VIEW(items[from[right]]), CROFT_VIEW(items[from[left]])) < 0"
                .to_string()
        } else {
            "items[from[right]] < items[from[left]]".to_string()
        };

        sorts.push((name.clone(), c_type, comes_first));

        name
    }

    /// Every sort function asked for.
    pub fn sort_functions(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();

        for (name, c_type, comes_first) in self.sorts.borrow().iter() {
            let signature = format!(
                "void {name}(const {c_type} *items, croft_u16 length, croft_u16 *order, croft_u16 *room)"
            );

            prototypes.push(format!("{signature};"));
            out.push_str(&format!(
                "{signature}\n{{\n{}}}\n\n",
                sort_body(comes_first)
            ));
        }

        out
    }
}

/// The body of a sort function: a bottom-up merge sort of the items'
/// places, O(n log n). Runs of `width` places are merged in pairs from
/// `from` into `to`, which then swap, with `width` doubling each time. An
/// item from the right run goes first only when it truly comes first, so
/// items that are the same keep their order. `comes_first` is true when the
/// item at `from[right]` comes before the one at `from[left]`.
///
/// Every sum stays within `length`, so no count wraps around even when a
/// list holds 65535 items.
fn sort_body(comes_first: &str) -> String {
    format!(
        "    croft_u16 width;
    croft_u16 start;
    croft_u16 middle;
    croft_u16 end;
    croft_u16 left;
    croft_u16 right;
    croft_u16 out;
    croft_u16 *from;
    croft_u16 *to;
    croft_u16 *swap;

    for (out = 0; out < length; out++) {{
        order[out] = out;
    }}

    from = order;
    to = room;

    for (width = 1; width < length; width = width > length - width ? length : (croft_u16) (width + width)) {{
        for (start = 0; start < length; start = end) {{
            middle = length - start > width ? (croft_u16) (start + width) : length;
            end = length - middle > width ? (croft_u16) (middle + width) : length;
            left = start;
            right = middle;

            for (out = start; out < end; out++) {{
                if (right < end && (left == middle || {comes_first})) {{
                    to[out] = from[right++];
                }} else {{
                    to[out] = from[left++];
                }}
            }}
        }}

        swap = from;
        from = to;
        to = swap;
    }}

    if (from != order) {{
        for (out = 0; out < length; out++) {{
            order[out] = from[out];
        }}
    }}
"
    )
}
