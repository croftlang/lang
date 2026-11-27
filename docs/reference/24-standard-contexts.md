# Standard contexts

Two standard contexts need no import: Std.Console, with the `Console`, `Arguments`, `Environment` and `Process` ports, and Std.Storage, for repositories. The rest of the built-ins are imported, like names from any other context, so each context says what it relies on and the standard library can grow without filling every namespace. An import covers every file of the context, and needs no arrow in the domain's contexts. Std.Files, Std.Http, Std.Random and Std.Time are written in dn: a port, its errors, and an adapter that dn.toml can bind, StdioFiles on any machine with C's stdio, CurlHttp wherever libcurl is, which also brings HTTPS, SeededRandom on any machine, and SystemClock wherever C's time and localtime are. CurlHttp speaks only http and https, redirects included, and follows at most 10 redirects, giving the last answer's status, headers and body; any other URL, such as `file:///etc/passwd`, and an eleventh redirect fail with `Unreachable`. Each request takes a URL of up to 1024 characters and up to 8 headers, `[:]` for none, which CurlHttp sends with every redirect, leaving out `Authorization` and `Cookie` when a redirect leads to another host; a header whose name or value holds a carriage return, a line feed or a NUL, or whose name is empty or holds a colon, fails the request with `Unreachable` before anything is sent. SeededRandom is a xorshift32 generator, seeded once from C's `time` the first time it is used, so each run gives different numbers, and a machine without a clock the same ones; it is fine for games and samples and never for anything secret. Std.Time's `Date` is a day on the Gregorian calendar, carried back before 1582, from year 1 to 9999, and only ever one its month has: its invariant `DayInMonth` makes `Date { year: 2025, month: 2, day: 29 }` an error from `croft check`, and with checks on such a date made while the program runs stops it; `days_between` and `add_days` count leap years, and with checks on a date past either end stops the program. SystemClock reads the machine's own time zone; a machine with no clock gives 1 January 1970 at midnight. Whatever is imported, the generated C only carries the runtime functions the program calls.

```croft
context Notes

use { trim, split, to_upper, to_bytes, to_text, replace, join, code_at, from_code, to_hex, from_hex, to_decimal } from Std.Text
use { Decimal, max, min, abs, clamp, pow, sign, checked_add } from Std.Numbers
use { Files, FileError } from Std.Files

// Std.Console, with no import: Console, Arguments, Environment and Process
Console.print("hello")   Console.print_error("no such room")
Console.read_line()   // Option<String>: None at the end of the input
Arguments.all()   // List<String>
Environment.get("HOME")   // Option<String>: None when it isn't set
Process.exit(1)   // stops the program with an exit code from 0 to 255

// Std.Text: strings, and text to and from bytes
name.contains("Sir")   name.starts_with("Sir")   name.ends_with("!")
name.to_upper()   name.to_lower()
name.trim()   // without the spaces, tabs, "\r" and "\n" at either end
name.slice(0, 3)   line.split(",")
text.to_int()   // Option<Int>: digits with an optional "-", None past Int's range
name.index_of("Sir")   // Option<Int>
text.lines()   // without "\n" or "\r\n", and no empty line after the last
html.replace("&", "&amp;")   "-".repeat(8)
score.to_string().pad_left(5, "0")   name.pad_right(10, " ")
names.join(", ")   // a List<String>
"DN".to_bytes()   data.to_text()
name.code_at(0)   // Option<Int>: the character's code from 0 to 255, None past the end
from_code(65)   // "A"; a code that isn't from 0 to 255 is a mistake
255.to_hex()   // "FF": capitals, no prefix; (-1).to_hex() is "FFFFFFFF", its 32 bits
from_hex("ff")   // Some(255): either case, no prefix or sign, None past 32 bits
let price: Option<Decimal<2>> = "12.5".to_decimal()   // Some(12.50): the places come from where it goes

// Std.Numbers
max(a, b)   min(a, b)   abs(n)   Decimal<2>
clamp(volume, 0, 11)   // volume kept from 0 to 11; a low above the high is a mistake
pow(2, 10)   // 1024; an exponent below zero, or an answer past 32 bits, is a mistake
sign(n)   // -1, 0 or 1
checked_add(a, b)   checked_subtract(a, b)   checked_multiply(a, b)   // Option<Int>: None past 32 bits

// Std.Collections: Map<K, V> and Set<T>, as many entries as their Length constraints allow
use { Map, Set } from Std.Collections
let stock: Map<String, Int> = ["sword": 1, "key": 2]
stock.get("key")            // Some(2)
stock.set("map", 1)         // a copy with map added
stock.remove("sword")       // a copy without sword
stock.contains("key")   stock.length   stock.is_empty   stock.keys   stock.values
let seen: Set<Int> = [3, 1]   // or [] when empty; an item listed twice is a mistake
seen.add(2)                 // a copy with 2 added; adding 3 again changes nothing
seen.remove(3)              // a copy without 3
seen.contains(1)   seen.length   seen.is_empty
seen.items                  // List<Int>: [3, 1, 2], in the order they were added

// Std.Encoding: bytes as text and back, text for URLs, and checksums, here
// in a context of its own, since from_hex comes from Std.Text or Std.Encoding
context Packets
use { to_hex, from_hex, to_base64, from_base64, percent_encode, percent_decode, crc16, crc32 } from Std.Encoding
let header: Bytes = [0x44, 0x4E, 0xFF]
header.to_hex()            // "444EFF"
from_hex("444eff")         // Some([0x44, 0x4E, 0xFF]); None for "444", "0x44" or "44 4E"
header.to_base64()         // "RE7/"
from_base64("RE7/")        // Some([0x44, 0x4E, 0xFF]); None for anything but base64
"fish & chips".percent_encode()     // "fish%20%26%20chips"
"fish%20%26%20chips".percent_decode()   // Some("fish & chips"); None for "100%"
"123456789".to_bytes().crc16()   // 0x29B1, CRC-16/CCITT-FALSE
"123456789".to_bytes().crc32()   // 0xCBF43926 as 32 bits: -873187034

// Std.Files: read, read_part, size, write, append, exists, remove, list, make_folder, copy, rename
service Keep(): Result<Unit, FileError> {
    calls { Files }
} = {
    Files.write("notes.txt", "first line".to_bytes())?
    let data = Files.read("notes.txt", 4096)?   // at most 4096 bytes; a longer file fails with NoRoom
    let header = Files.read_part("save.bin", 16, 8)?   // up to 8 bytes, skipping the first 16; none past the end
    let bytes = Files.size("logo.png")?   // however large, up to 2147483647
    let pages = Files.list("docs")?   // ["a.md", "b.md", "drafts/"], in byte order
    Files.make_folder("site/blog")?    // and site, if it isn't there yet
    Files.copy("logo.png", "site/logo.png")?   // any size, not only what a Bytes holds
    Files.rename("notes.txt", "site/notes.txt")?   // moved, on the same disk
    ...
}

// Std.Http: get(url, headers), post(url, headers, body), put(url, headers, body),
// delete(url, headers), each a Response { status, headers, body } or an HttpError,
// the body holding at most the target's `bytes_length`, or failing with TooLarge
use { Response, HttpError } from Std.Http

service Upload(save: Bytes): Result<Unit, HttpError> {
    calls { Http } from Std.Http
} = {
    let response = Http.post("https://example.com/saves", ["Content-Type": "application/octet-stream"], save)?
    let list = Http.get("https://example.com/saves", [:])?   // no headers
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
    // `Date { year: 2025, month: 2, day: 29 }` doesn't compile: it breaks `DayInMonth`
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

Std.Encoding works on `Bytes`. The `to_` names, `percent_encode`, `percent_decode`, `crc16` and `crc32` are methods of what they read, as `to_bytes` is in Std.Text: `to_hex`, `to_base64`, `crc16` and `crc32` of `Bytes`, and the percent methods of a `String`. `from_hex` and `from_base64` are functions taking the text, since they make `Bytes` from a `String`. `to_hex` writes two upper-case digits for each byte, and `from_hex` reads either case, two digits for each byte, with nothing between them. Base64 is RFC 4648's standard alphabet with `=` padding; `from_base64` gives `None` for a length that isn't a multiple of 4, a character outside the alphabet, `=` anywhere but the end, or leftover bits that aren't 0, so each `Bytes` has one base64 text. `percent_encode` keeps letters, digits and `-._~`, RFC 3986's unreserved characters, and writes every other character as `%` and two upper-case hex digits, each byte of a character on its own; `percent_decode` reads either case, gives `None` for a `%` not followed by two hex digits, and leaves `+` as it is. `crc16` is CRC-16/CCITT-FALSE, polynomial 0x1021 starting from 0xFFFF, and `crc32` is the CRC-32 of zip, PNG and Ethernet, its 32 bits given as an `Int`, so one with the top bit set is below zero. Text is checked through its bytes: `text.to_bytes().crc32()`. Each result has room for all of it, two characters for each byte of hex, four for each three of base64 and three for each character percent-encoded, up to what a string or `Bytes` with no `Length` holds; a longer result stops the program, or is cut short when the target sets `checks = false`, as a string is.

`to_hex` and `from_hex` are in both Std.Text and Std.Encoding. `to_hex` is a method, so the value it is called on picks one: `255.to_hex()` is Std.Text's and `header.to_hex()` is Std.Encoding's, and a context that uses both imports `to_hex` from each. `from_hex` is a function, so the import decides: from Std.Text it gives an `Option<Int>`, from Std.Encoding an `Option<Bytes>`. A context imports it from one of them; importing it from both is an error, `` `from_hex` can't come from both `Std.Text` and `Std.Encoding`: they mean different things, so import it from one ``. Two contexts of one program can each import their own.
