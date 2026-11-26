# Standard contexts

Two standard contexts need no import: Std.Console, with the `Console` and `Arguments` ports, and Std.Storage, for repositories. The rest of the built-ins are imported, like names from any other context, so each context says what it relies on and the standard library can grow without filling every namespace. An import covers every file of the context, and needs no arrow in the domain's contexts. Std.Files and Std.Http are written in dn: a port, its errors, and an adapter that dn.toml can bind, StdioFiles on any machine with C's stdio and CurlHttp wherever libcurl is, which also brings HTTPS. Whatever is imported, the generated C only carries the runtime functions the program calls.

```croft
context Notes

use { trim, split, to_upper, to_bytes, to_text } from Std.Text
use { Decimal, max, min, abs } from Std.Numbers
use { Files, FileError } from Std.Files

// Std.Text: strings, and text to and from bytes
name.contains("Sir")   name.starts_with("Sir")   name.ends_with("!")
name.to_upper()   name.to_lower()   name.trim()
name.slice(0, 3)   line.split(",")   text.to_int()   // Option<Int>
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

// Std.Files: read, write, append, exists, remove, list
service Keep(): Result<Unit, FileError> {
    calls Files
} = {
    files.write("notes.txt", "first line".to_bytes())?
    let data = files.read("notes.txt")?
    let pages = files.list("docs")?   // ["a.md", "b.md", "drafts/"], in byte order
    ...
}

// Std.Http: get, post, put, delete, each a Response { status, headers, body } or an HttpError
use { Http, Response, HttpError } from Std.Http

service Upload(save: Bytes): Result<Unit, HttpError> {
    calls Http
} = {
    let response = http.post("https://example.com/saves", save)?
    ...
}

// dn.toml
// [target.desktop.adapters]
// Files = "StdioFiles"
// Http = "CurlHttp"          // link with -lcurl

// without the import:
// `Files` needs `use { Files } from Std.Files`
```
