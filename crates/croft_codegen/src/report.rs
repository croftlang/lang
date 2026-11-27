//! `croft size`: how much memory a program's types and functions take on
//! one target, worked out from the C it generates, and where to shrink it.

use std::collections::{BTreeMap, BTreeSet};

use croft_analysis::symbols::SymbolKind;
use croft_analysis::{Program, Type};

use crate::types::memory::POINTER_SIZE;
use crate::Generator;

/// How many of the largest function frames the report lists.
const FRAMES_SHOWN: usize = 10;

/// The report for one target, as text to print.
pub fn size_report(program: &Program, target: &str) -> Result<String, String> {
    let generator = Generator::new(program, target)?;
    let files = generator.generate()?;

    let c = files
        .iter()
        .find(|file| file.name == "program.c")
        .map_or("", |file| file.contents.as_str());

    let mut out = format!(
        "Sizes for target `{target}`, in bytes, counted as an 8-bit compiler lays them out: \
         no padding, {POINTER_SIZE}-byte pointers, 4-byte whole numbers.\n"
    );

    out.push_str(&generator.kept_section());
    out.push_str(&generator.frames_section(c));
    out.push_str(&generator.strings_section());
    out.push_str(&generator.bytes_section());
    out.push_str(&generator.lists_section());
    out.push_str(&generator.numbers_section());

    Ok(out)
}

impl<'a> Generator<'a> {
    /// Each aggregate, entity and value, in memory and, for a repository's
    /// aggregate, saved.
    fn kept_section(&self) -> String {
        let mut rows = Vec::new();

        for kind in [SymbolKind::Aggregate, SymbolKind::Entity, SymbolKind::Value] {
            for named in self.model.all(kind) {
                if named.context.starts_with("Std.") {
                    continue;
                }

                let ty = Type::Named(named.clone());
                let saved = (kind == SymbolKind::Aggregate
                    && self
                        .storages
                        .keys()
                        .any(|r| self.model.repository_aggregate(r).as_ref() == Some(&named)))
                .then(|| 4 + self.serialized_size(&ty));

                rows.push((
                    format!("{}.{}", named.context, named.name),
                    self.memory_size(&ty),
                    saved,
                ));
            }
        }

        if rows.is_empty() {
            return String::new();
        }

        rows.sort_by_key(|row| std::cmp::Reverse(row.1));
        let mut out = "\nKept\n".to_string();

        for (name, size, saved) in rows {
            let saved = saved.map_or(String::new(), |s| format!("  (saved: {s})"));
            out.push_str(&format!("  {size:>6}  {name}{saved}\n"));
        }

        out
    }

    /// The functions whose own variables take the most stack.
    fn frames_section(&self, c: &str) -> String {
        let mut frames = frames(c, |c_type| self.c_type_size(c_type));
        frames.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let mut out = "\nLargest stack frames (variables only)\n".to_string();

        for (name, size) in frames.iter().take(FRAMES_SHOWN) {
            out.push_str(&format!("  {size:>6}  {name}\n"));
        }

        out
    }

    /// How many characters each string holds: those kept in sizes of their
    /// own, and one with nothing to bound it.
    fn strings_section(&self) -> String {
        let capacity = self.sizes.string_capacity;

        let bounded: Vec<String> = self.texts.borrow().iter().map(i64::to_string).collect();

        let sizes = if bounded.is_empty() {
            String::new()
        } else {
            format!(
                "  Strings kept in sizes of their own hold {} characters.\n",
                bounded.join(", ")
            )
        };

        format!(
            "\nStrings\n  Each string is kept in the most characters it can need, from the \
             literals and `Length`s it comes from.\n{sizes}  A string with nothing to bound it, \
             such as a field with no `Length`, holds {capacity} characters ({} bytes), the \
             target's `strings_length`.\n",
            2 + capacity
        )
    }

    /// How many values each `Bytes` holds, as for strings. Says nothing for a
    /// program that uses no `Bytes`.
    fn bytes_section(&self) -> String {
        let bounded: Vec<String> = (self.byte_sizes.borrow().iter())
            .map(i64::to_string)
            .collect();

        let full = self.used_types.borrow().0.contains(&Type::Bytes);

        if bounded.is_empty() && !full {
            return String::new();
        }

        let capacity = self.sizes.bytes_capacity;

        let sizes = if bounded.is_empty() {
            String::new()
        } else {
            format!(
                "  Bytes kept in sizes of their own hold {} values.\n",
                bounded.join(", ")
            )
        };

        format!(
            "\nBytes\n  Each `Bytes` is kept in the most values it can need, from the literals \
             and `Length`s it comes from.\n{sizes}  One with nothing to bound it holds \
             {capacity} values ({} bytes).\n",
            2 + capacity
        )
    }

    /// How many items the lists of each item type hold: those kept in
    /// sizes of their own, and one with nothing to bound it. Says nothing
    /// for a program that uses no lists.
    fn lists_section(&self) -> String {
        let mut items: BTreeMap<String, (i64, BTreeSet<i64>)> = BTreeMap::new();

        for ty in self.used_types.borrow().0.iter() {
            let (item, sized) = match ty {
                Type::List(item) => (item, None),
                Type::SizedList(item, length) => (item, Some(i64::from(*length))),
                _ => continue,
            };

            let entry = (items.entry(format!("List<{item}>")))
                .or_insert_with(|| (self.sizes.list_capacity(item), BTreeSet::new()));

            entry.1.extend(sized);
        }

        if items.is_empty() {
            return String::new();
        }

        let lines: String = (items.iter())
            .map(|(list, (capacity, sized))| {
                let sized: Vec<String> = sized.iter().map(i64::to_string).collect();

                let sized = if sized.is_empty() {
                    String::new()
                } else {
                    format!("sized ones hold {} items; ", sized.join(", "))
                };

                format!("  {list}: {sized}one with nothing to bound it holds {capacity}.\n")
            })
            .collect();

        format!(
            "\nLists\n  Each list is kept in the most items it can need, from the literals \
             and `Length`s it comes from.\n{lines}"
        )
    }

    /// Whole-number fields with no `Range`, each kept in four bytes.
    fn numbers_section(&self) -> String {
        let mut fields = Vec::new();

        for kind in [SymbolKind::Aggregate, SymbolKind::Entity, SymbolKind::Value] {
            for named in self.model.all(kind) {
                if named.context.starts_with("Std.") {
                    continue;
                }

                for (field, ty, c) in self.struct_fields(&named) {
                    if ty == Type::Int && c == "croft_int" {
                        fields.push(format!("{}.{field}", named.name));
                    }
                }
            }
        }

        if fields.is_empty() {
            return String::new();
        }

        let mut out =
            "\nWhole numbers with no Range, 4 bytes each; a Range up to 255 makes one 1 byte\n"
                .to_string();

        for field in fields {
            out.push_str(&format!("  {field}\n"));
        }

        out
    }
}

/// Each function in generated C with the bytes of the variables it
/// declares at its top, using `size` for each C type: a pointer takes
/// `POINTER_SIZE` bytes, and an array such as `croft_u16 order[500]` as
/// many items as it holds.
fn frames(c: &str, size: impl Fn(&str) -> Option<i64>) -> Vec<(String, i64)> {
    let lines: Vec<&str> = c.lines().collect();
    let mut frames = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let is_start = (line.starts_with("void ") || line.starts_with("static "))
            && lines.get(index + 1) == Some(&"{");

        if !is_start {
            continue;
        }

        let Some(name) = line
            .split('(')
            .next()
            .and_then(|head| head.split_whitespace().last())
        else {
            continue;
        };

        let mut total = 0;

        for declaration in &lines[index + 2..] {
            let Some(declaration) = declaration
                .strip_prefix("    ")
                .and_then(|d| d.strip_suffix(';'))
            else {
                break;
            };

            let parts: Vec<&str> = declaration.split_whitespace().collect();

            let [c_type, variable] = parts.as_slice() else {
                break;
            };

            if variable.contains(['(', '=']) {
                break;
            }

            total += variable_size(variable, size(c_type).unwrap_or(0));
        }

        frames.push((name.trim_start_matches('*').to_string(), total));
    }

    frames
}

/// The bytes a variable declared as `variable`, such as `t`, `*from` or
/// `order[500]`, takes when one of its C type takes `item` bytes.
fn variable_size(variable: &str, item: i64) -> i64 {
    if variable.starts_with('*') {
        return POINTER_SIZE;
    }

    let Some((_, length)) = variable.split_once('[') else {
        return item;
    };

    let count = length
        .strip_suffix(']')
        .and_then(|count| count.parse::<i64>().ok())
        .unwrap_or(1);

    item * count
}

/// Tests for working out function frame sizes.
#[cfg(test)]
mod tests {
    use super::frames;

    /// Each function's frame is the sum of the variables declared at its top.
    #[test]
    fn adds_up_the_variables_at_the_top_of_each_function() {
        let c = "void Shop_open(croft_u8 p_day, croft_bool *out)\n{\n    croft_int t;\n    croft_u8 t_2;\n\n    t = 1;\n}\n\n\
                 static croft_bool croft_check(void)\n{\n    return 1;\n}\n";

        let sizes = |c_type: &str| match c_type {
            "croft_int" => Some(4),
            "croft_u8" => Some(1),
            _ => None,
        };

        assert_eq!(
            frames(c, sizes),
            vec![("Shop_open".to_string(), 5), ("croft_check".to_string(), 0)]
        );
    }

    /// An array counts each item it holds, and a pointer its own size
    /// whatever it points to, as a sort's index arrays and its places do.
    #[test]
    fn counts_each_item_of_an_array_and_a_pointer_as_a_pointer() {
        let c = "void Shop_sort(croft_list_int *out)\n{\n    croft_u16 index;\n    \
                 croft_u16 order[500];\n    croft_int keys[500];\n    croft_int *from;\n\n    index = 0;\n}\n";

        let sizes = |c_type: &str| match c_type {
            "croft_int" => Some(4),
            "croft_u16" => Some(2),
            _ => None,
        };

        assert_eq!(
            frames(c, sizes),
            vec![(
                "Shop_sort".to_string(),
                2 + 1000 + 2000 + super::POINTER_SIZE
            )]
        );
    }
}
