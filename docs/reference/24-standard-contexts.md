# Standard contexts

Two standard contexts need no import: Std.Console, with the `Console`, `Arguments` and `Process` ports, and Std.Storage, for repositories. The rest of the built-ins are imported, like names from any other context, so each context says what it relies on and the standard library can grow without filling every namespace. An import covers every file of the context, and needs no arrow in the domain's contexts. Std.Files, Std.Http, Std.Random and Std.Time are written in dn: a port, its errors, and an adapter that dn.toml can bind, StdioFiles on any machine with C's stdio, CurlHttp wherever libcurl is, which also brings HTTPS, SeededRandom on any machine, and SystemClock wherever C's time and localtime are. CurlHttp speaks only http and https, redirects included, and follows at most 10 redirects, giving the last answer's status, headers and body; any other URL, such as `file:///etc/passwd`, and an eleventh redirect fail with `Unreachable`. SeededRandom is a xorshift32 generator, seeded once from C's `time` the first time it is used, so each run gives different numbers, and a machine without a clock the same ones; it is fine for games and samples and never for anything secret. Std.Time's `Date` is a day on the Gregorian calendar, carried back before 1582, from year 1 to 9999; `days_between` and `add_days` count leap years, and with checks on a date past either end stops the program. SystemClock reads the machine's own time zone; a machine with no clock gives 1 January 1970 at midnight. Whatever is imported, the generated C only carries the runtime functions the program calls.

```croft
context Notes

use { trim, split, to_upper, to_bytes, to_text, replace, join } from Std.Text
use { Decimal, max, min, abs, clamp, pow, sign, checked_add } from Std.Numbers
use { Files, FileError } from Std.Files

// Std.Console, with no import: Console, Arguments and Process
Console.print("hello")   Console.print_error("no such room")
Console.read_line()   // Option<String>: None at the end of the input
Arguments.all()   // List<String>
Process.exit(1)   // stops the program with an exit code from 0 to 255

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
clamp(volume, 0, 11)   // volume kept from 0 to 11; a low above the high is a mistake
pow(2, 10)   // 1024; an exponent below zero, or an answer past 32 bits, is a mistake
sign(n)   // -1, 0 or 1
checked_add(a, b)   checked_subtract(a, b)   checked_multiply(a, b)   // Option<Int>: None past 32 bits

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

// Std.Time: Date { year, month, day }, Time { hour, minute, second }, and the Clock port
use { Date, Time, Clock, days_between, add_days } from Std.Time

service Due(): Int {
    calls { Clock }
} = {
    let today = Clock.today()   // Date { year: 2026, month: 11, day: 27 }
    let now = Clock.now()       // Time { hour: 9, minute: 30, second: 0 }
    let due = add_days(today, 30)   // Date { year: 2026, month: 12, day: 27 }
    add_days(Date { year: 2024, month: 2, day: 28 }, 1)   // 29 February 2024, a leap year
    days_between(today, Date { year: 2027, month: 1, day: 1 })   // 35, or negative when earlier
}

// dn.toml
// [target.desktop.adapters]
// Files = "StdioFiles"
// Http = "CurlHttp"          // link with -lcurl
// Random = "SeededRandom"
// Clock = "SystemClock"

// without the import:
// `Files` needs `use { Files } from Std.Files`
```
