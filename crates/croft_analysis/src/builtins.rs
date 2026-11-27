//! What the built-in types, functions and constraints are, for editors to
//! show. The checks in `check/types.rs`, `check/body/builtin.rs` and
//! `check/constraints.rs` decide what each one takes and gives back; when one
//! is added there, add it here too.

/// How a built-in type, function or constraint is written, and what it is.
pub fn built_in_doc(name: &str) -> Option<(&'static str, &'static str)> {
    let found = match name {
        "Int" => (
            "Int",
            "A whole number of at least 32 bits. Where its values are known, \
             from a `Range`, a length or arithmetic on them, it is kept in the \
             smallest type that holds them; the answer is always the one 32 \
             bits would give.",
        ),
        "Float" => (
            "Float",
            "A floating-point number, kept in 64 bits. `Decimal` is exact, and \
             works on machines without floating point.",
        ),
        "Decimal" => (
            "Decimal<N>",
            "A fixed-point number with `N` places after the point: exact, and \
             portable to machines without floating point. An `Int` works \
             anywhere a `Decimal` does. Multiplying and dividing round half \
             to even.",
        ),
        "Bool" => ("Bool", "`true` or `false`."),
        "String" => (
            "String",
            "Text. A string with a `Length` holds that many characters. One \
             without holds 32, or more when a `Length` elsewhere in the program \
             allows a longer string. Longer text is cut short.",
        ),
        "Bytes" => (
            "Bytes",
            "A sequence of bytes, each from 0 to 255, for file contents, saves \
             and network messages. Written as a list of numbers where `Bytes` \
             is expected: `[0x44, 0x4E, 1]`.",
        ),
        "Unit" => (
            "Unit",
            "No value: what a function, service or port operation with no \
             return type gives back.",
        ),
        "List" => (
            "List<T>",
            "Items of one type, in order. A list holds as many as the largest \
             `Length` on its type allows, 16 when there is none.",
        ),
        "Map" => (
            "Map<K, V>",
            "Keys, each with a value. `get` gives an `Option` of a key's value; \
             `set` and `remove` give a changed copy. A map holds as many entries \
             as the largest `Length` on its type allows, 16 when there is none.",
        ),
        "Set" => (
            "Set<T>",
            "Items of one type, each held once. `add` and `remove` give a changed \
             copy, `contains` says whether an item is there and `items` lists \
             them in the order they were added. A set holds as many items as \
             the largest `Length` on its type allows, 16 when there is none.",
        ),
        "Option" => (
            "Option<T>",
            "A value that may be missing: `Some(value)` or `None`.",
        ),
        "Result" => (
            "Result<T, E>",
            "What a body that can fail gives back: `Ok(value)` or `Err(error)`. \
             `?` passes a failure up; a body whose last line is a plain value \
             counts as success.",
        ),
        "Range" => (
            "constraint Range(number: Int, low: Int, high: Int)",
            "The number is from `low` to `high`, both included. On a field, \
             `where Range(0, 100)` also sizes the number: it is kept in as few \
             bytes as hold every value it allows.",
        ),
        "Length" => (
            "constraint Length(items: String or List<T>, low: Int, high: Int)",
            "The string or list holds from `low` to `high` items, both included. \
             `Length(4)`, with one argument, means exactly 4. On a string field, \
             `where Length(1, 12)` also sizes the string: it holds 12 characters, \
             and longer text is cut short.",
        ),
        "max" => (
            "fn max(a: Number, b: Number): Number",
            "The larger of two numbers of the same type.",
        ),
        "min" => (
            "fn min(a: Number, b: Number): Number",
            "The smaller of two numbers of the same type.",
        ),
        "abs" => (
            "fn abs(number: Int): Int",
            "How far the number is from zero.",
        ),
        "clamp" => (
            "fn clamp(number: Number, low: Number, high: Number): Number",
            "The number kept from `low` to `high`: `low` when it is below, \
             `high` when it is above. A `low` above `high` is a mistake.",
        ),
        "pow" => (
            "fn pow(base: Int, exponent: Int): Int",
            "The base multiplied by itself `exponent` times; 1 when the \
             exponent is 0. An exponent below zero, and an answer past 32 \
             bits, are mistakes, as for `*`.",
        ),
        "sign" => (
            "fn sign(number: Number): Int",
            "-1 when the number is below zero, 0 when it is zero, and 1 \
             when it is above.",
        ),
        "checked_add" => (
            "fn checked_add(a: Int, b: Int): Option<Int>",
            "`Some(a + b)`, or `None` when the answer doesn't fit in 32 bits.",
        ),
        "checked_subtract" => (
            "fn checked_subtract(a: Int, b: Int): Option<Int>",
            "`Some(a - b)`, or `None` when the answer doesn't fit in 32 bits.",
        ),
        "checked_multiply" => (
            "fn checked_multiply(a: Int, b: Int): Option<Int>",
            "`Some(a * b)`, or `None` when the answer doesn't fit in 32 bits.",
        ),
        "from_code" => (
            "fn from_code(code: Int): String",
            "The one character whose code is `code`, from 0 to 255. Any other \
             code is a mistake.",
        ),
        "from_hex" => (
            "fn from_hex(text: String): Option<Int>",
            "The whole number written in hexadecimal, in either case, with no \
             prefix or sign: `from_hex(\"FF\")` is `Some(255)`. Eight digits \
             give the 32 bits of an `Int`, so `\"FFFFFFFF\"` is -1. `None` \
             for anything else, and past 32 bits.",
        ),
        "Some" => (
            "fn Some(value: T): Option<T>",
            "An `Option` that holds the value.",
        ),
        "None" => ("None: Option<T>", "An `Option` that holds nothing."),
        "Ok" => (
            "fn Ok(value: T): Result<T, E>",
            "A `Result` that succeeded, with the value.",
        ),
        "Err" => (
            "fn Err(error: E): Result<T, E>",
            "A `Result` that failed, with the error.",
        ),
        _ => return None,
    };

    Some(found)
}
