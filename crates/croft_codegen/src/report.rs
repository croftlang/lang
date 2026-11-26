//! `croft size`: how much memory a program's types and functions take on
//! one target, worked out from the C it generates, and where to shrink it.

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

    fn strings_section(&self) -> String {
        let capacity = self.sizes.string_capacity;

        let from = match &self.sizes.string_capacity_from {
            Some(label) => format!("set by the `Length` on {label}"),
            None => "the default".to_string(),
        };

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
             such as a parameter with no `Length`, holds {capacity} characters ({} bytes), \
             {from}.\n",
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
/// declares at its top, using `size` for each C type.
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

            total += size(c_type).unwrap_or(0);
        }

        frames.push((name.trim_start_matches('*').to_string(), total));
    }

    frames
}

#[cfg(test)]
mod tests {
    use super::frames;

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
}
