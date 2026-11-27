# Standard contexts

Two standard contexts need no import: Std.Console, with the `Console` and `Arguments` ports, and Std.Storage, for repositories. The rest of the built-ins are imported, like names from any other context, so each context says what it relies on and the standard library can grow without filling every namespace. An import covers every file of the context, and needs no arrow in the domain's contexts. Std.Files, Std.Http and Std.Random are written in dn: a port, its errors, and an adapter that dn.toml can bind, StdioFiles on any machine with C's stdio, CurlHttp wherever libcurl is, which also brings HTTPS, and SeededRandom on any machine. CurlHttp speaks only http and https, redirects included, and follows at most 10 redirects, giving the last answer's status, headers and body; any other URL, such as `file:///etc/passwd`, and an eleventh redirect fail with `Unreachable`. SeededRandom is a xorshift32 generator, seeded once from C's `time` the first time it is used, so each run gives different numbers, and a machine without a clock the same ones; it is fine for games and samples and never for anything secret. Whatever is imported, the generated C only carries the runtime functions the program calls.

```croft
context Notes

use { trim, split, to_upper, to_bytes, to_text, replace, join } from Std.Text
use { Decimal, max, min, abs } from Std.Numbers
use { Files, FileError } from Std.Files

// Std.Text: strings, and text to and from bytes
name.contains("Sir")   name.starts_with("Sir")   name.ends_with("!")
name.to_upper()   name.to_lower()   name.trim()
name.slice(0, 3)   line.split(",")
text.to_int()   // Option<Int>: digits with an optional "-", None past Int's range
name.index_of("Sir")   // Option<Int>
text.lines()   // without "\n" or "\r\n", and no empty line after the last
html.replace("&", "&amp;")   "-".repeat(8)
score.to_string().pad_left(5, "0")   name.pad_right(10, " ")
names.join(", ")   // a List<String>
"DN".to_bytes()   data.to_text()

// Std.Numbers
max(a, b)   min(a, b)   abs(n)   Decimal<2>

// Std.Collections: Map<K, V>, as many entries as its Length constraint allows
use { Map } from Std.Collections
let stock: Map<String, Int> = ["sword": 1, "key": 2]
stock.get("key")            // Some(2)
stock.set("map", 1)         // a copy with map added
stock.remove("sword")       // a copy without sword
stock.contains("key")   stock.length   stock.is_empty   stock.keys   stock.values

// Std.Files: read, write, append, exists, remove, list, make_folder, copy
service Keep(): Result<Unit, FileError> {
    calls { Files }
} = {
    Files.write("notes.txt", "first line".to_bytes())?
    let data = Files.read("notes.txt")?
    let pages = Files.list("docs")?   // ["a.md", "b.md", "drafts/"], in byte order
    Files.make_folder("site/blog")?    // and site, if it isn't there yet
    Files.copy("logo.png", "site/logo.png")?   // any size, not only what a Bytes holds
    ...
}

// Std.Http: get, post, put, delete, each a Response { status, headers, body } or an HttpError
use { Response, HttpError } from Std.Http

service Upload(save: Bytes): Result<Unit, HttpError> {
    calls { Http } from Std.Http
} = {
    let response = Http.post("https://example.com/saves", save)?
    ...
}

// Std.Random: between(low, high), a whole number with both ends included
use { Random } from Std.Random

service Roll(): Int {
    calls { Random }
} = Random.between(1, 6)   // `Random.between(6, 1)` doesn't compile: it breaks `low <= high`

// dn.toml
// [target.desktop.adapters]
// Files = "StdioFiles"
// Http = "CurlHttp"          // link with -lcurl
// Random = "SeededRandom"

// without the import:
// `Files` needs `use { Files } from Std.Files`
```
