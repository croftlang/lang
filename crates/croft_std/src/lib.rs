//! The standard contexts written in Croft, such as `Std.Files`, each kept
//! beside the C its adapters call. The checker reads their sources like any
//! other context's, code generation writes out the C of each adapter a target
//! binds, and the language server opens the sources for an editor.
//!
//! The contexts built into the compiler, such as `Std.Text`, are not here:
//! the checker knows their types and code generation writes their C.

/// A standard context written in Croft.
pub struct Context {
    /// The name it is imported by, such as `Std.Files`.
    pub name: &'static str,
    /// Its Croft source.
    pub source: &'static str,
    /// Its adapters whose native functions are C kept here. An adapter whose
    /// C code generation writes, such as `MemoryStorage`, is not listed.
    pub adapters: &'static [Adapter],
}

/// An adapter of a standard context, and the C that holds its native functions.
pub struct Adapter {
    /// Its name, such as `StdioFiles`.
    pub name: &'static str,
    /// The file its C is written to, beside `program.c`, such as `croft_files.c`.
    pub file: &'static str,
    /// The C source of its native functions.
    pub c: &'static str,
    /// What a program using it is linked with, such as `-lcurl`.
    pub links: &'static [&'static str],
}

/// Every standard context written in Croft. A context's place in this list
/// gives its source id, so new contexts go on the end.
pub const CONTEXTS: [Context; 5] = [
    Context {
        name: "Std.Files",
        source: include_str!("../std/files/files.croft"),
        adapters: &[Adapter {
            name: "StdioFiles",
            file: "croft_files.c",
            c: include_str!("../std/files/stdio_files.c"),
            links: &[],
        }],
    },
    Context {
        name: "Std.Http",
        source: include_str!("../std/http/http.croft"),
        adapters: &[Adapter {
            name: "CurlHttp",
            file: "croft_http.c",
            c: include_str!("../std/http/curl_http.c"),
            links: &["-lcurl"],
        }],
    },
    Context {
        name: "Std.Storage",
        source: include_str!("../std/storage/storage.croft"),
        adapters: &[],
    },
    Context {
        name: "Std.Console",
        source: include_str!("../std/console/console.croft"),
        adapters: &[],
    },
    Context {
        name: "Std.Random",
        source: include_str!("../std/random/random.croft"),
        adapters: &[Adapter {
            name: "SeededRandom",
            file: "croft_random.c",
            c: include_str!("../std/random/seeded_random.c"),
            links: &[],
        }],
    },
];

/// The standard context called `name`, if there is one written in Croft.
pub fn context(name: &str) -> Option<&'static Context> {
    CONTEXTS.iter().find(|context| context.name == name)
}

/// The adapter called `name` in standard context `context`, when its C is kept here.
pub fn adapter(context: &str, name: &str) -> Option<&'static Adapter> {
    self::context(context)?
        .adapters
        .iter()
        .find(|adapter| adapter.name == name)
}

/// What a program is linked with for the C file called `file`, one of the
/// adapters' files: nothing for a file that is not.
pub fn links_for(file: &str) -> &'static [&'static str] {
    (CONTEXTS.iter())
        .flat_map(|context| context.adapters)
        .find(|adapter| adapter.file == file)
        .map_or(&[], |adapter| adapter.links)
}

/// Tests that the standard contexts and their C agree.
#[cfg(test)]
mod tests {
    use super::*;

    /// The body of `adapter Name for ...` in `source`, up to its closing brace.
    fn adapter_body<'s>(source: &'s str, name: &str) -> &'s str {
        let start = source
            .find(&format!("adapter {name} for "))
            .unwrap_or_else(|| panic!("no adapter {name}"));

        let open = start + source[start..].find('{').expect("its opening brace");
        let mut depth = 0;

        for (offset, c) in source[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => continue,
            }

            if depth == 0 {
                return &source[open..open + offset];
            }
        }

        panic!("adapter {name} has no closing brace")
    }

    /// The name of each native function `body` calls.
    fn natives(body: &str) -> Vec<&str> {
        body.split("native ")
            .skip(1)
            .map(|rest| rest.split('(').next().unwrap_or_default().trim())
            .collect()
    }

    /// Each native function an adapter calls is defined in its C, so a name
    /// changed on one side and not the other is found here, not when linking.
    #[test]
    fn defines_every_native_function_an_adapter_calls() {
        for context in &CONTEXTS {
            for adapter in context.adapters {
                let called = natives(adapter_body(context.source, adapter.name));
                assert!(
                    !called.is_empty(),
                    "{} calls no native functions",
                    adapter.name
                );

                for name in called {
                    assert!(
                        adapter.c.contains(&format!("\nvoid {name}(")),
                        "{}'s C in {} has no `void {name}(`",
                        adapter.name,
                        adapter.file
                    );
                }
            }
        }
    }

    /// Adapters with C kept here are found by name, with what they link with.
    #[test]
    fn finds_adapters_and_what_they_link_with() {
        assert_eq!(
            adapter("Std.Http", "CurlHttp").map(|a| a.file),
            Some("croft_http.c")
        );
        assert!(adapter("Std.Storage", "MemoryStorage").is_none());
        assert_eq!(links_for("croft_http.c"), ["-lcurl"]);
        assert!(links_for("croft_files.c").is_empty());
        assert_eq!(
            adapter("Std.Random", "SeededRandom").map(|a| a.file),
            Some("croft_random.c")
        );
        assert!(links_for("croft_random.c").is_empty());
        assert!(links_for("program.c").is_empty());
    }
}
