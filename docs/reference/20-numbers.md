# Numbers

Int is a whole number of exactly 32 bits on every machine, from -2147483648 to 2147483647. Where its values are known, from a Range on a field, parameter or loop counter, a length, or arithmetic on them, it is kept in the smallest type that holds them, and worked out in 16 bits when every value fits, as 8-bit machines do fastest; the answer is always the one 32 bits would give. `/` rounds toward zero, and `%` has the sign of the number divided. Arithmetic with no answer in 32 bits is a mistake: `+`, `-` or `*` past them, `-` of the lowest number or dividing it by -1, and `/` or `%` by zero. Where the values are known, `croft check` reports it, through calls to functions as well; otherwise the program stops with "a whole number doesn't fit in 32 bits" or "division by zero". A target with `checks = false` wraps the answer around in 32 bits instead, and dividing by zero gives 0. Decimal<2>, imported from Std.Numbers, is a fixed-point number with two decimal places, exact and portable to machines without floating point. It keeps a whole number of hundredths in the same 32 bits, so it holds up to 21474836.47, and an answer past that is a mistake in the same way, which stops the program with "a decimal doesn't fit in 32 bits". A literal such as 2.50 is a Decimal wherever one is expected, and an Int works anywhere a Decimal does. Adding and subtracting are exact; multiplying and dividing round half to even. Std.Numbers also has `clamp`, `pow` and `sign`. A `pow` past 32 bits is a mistake as `*` is, and so are an exponent below zero and a `clamp` whose low is above its high: `croft check` reports them when the values are known, and otherwise the program stops; with `checks = false`, `pow` gives 0 for an exponent below zero and `clamp` gives its low. `checked_add`, `checked_subtract` and `checked_multiply` give an `Option<Int>` instead, `None` when the answer doesn't fit in 32 bits, whatever the checks. Bit operators work on Int and bind tighter than comparisons, so `keys & FIRE != 0` tests the bit. Shifts go from 0 to 31. A let can name its type. Bytes holds values from 0 to 255, written as a list of numbers. One with a `Length` keeps room for that many, however many that is. One without, such as a field, a `let` that `set` changes, or an HTTP response's body, keeps room for the target's `bytes_length`, 64 unless croft.toml says otherwise, so one long `Length` doesn't make every Bytes as long. A longer value going into one stops the program, or is cut short when the target sets `checks = false`. A Bytes parameter borrows what it is given, so it takes one of any length. Std.Encoding writes Bytes as hex or base64 and reads them back, and gives their CRC-16 or CRC-32.

```croft
use { Decimal } from Std.Numbers

const FIRE: Int = 1 << 4
const MASK: Int = 0b0001_1111
const SCREEN: Int = 0x4000

fn fire_pressed(keys: Int): Bool = keys & FIRE != 0

fn flip(keys: Int): Int = ~keys & MASK

// -7 / 2 is -3, and -7 % 2 is -1
fn grow(n: Int): Int = n * 1000

// an error: `grow(5000000)` works out `5000000 * 1000`, which gives 5000000000
fn grown(): Int = grow(5000000)

fn with_tax(price: Decimal<2> where Range(0, 999.99)): Decimal<2> = price * 1.20

// 10.00 / 3 is 3.33; 0.05 / 2 rounds to 0.02, the even neighbour
fn share(total: Decimal<2>, people: Int): Decimal<2> = total / people

let header: Bytes = [0x44, 0x4E, 1]   // length, get, set, add, slice, and + to join
let tape = header.add_u16(6912)        // 0x1B00 as 00 1B, low byte first
tape.get_u16(3)                        // Some(6912); get_u16_be and add_u16_be for the 68000

let price: Decimal<2> = 4
price.to_string()  // "4.00"
price.round()      // 4, an Int
```
