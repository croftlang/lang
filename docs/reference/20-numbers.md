# Numbers

Int is a whole number of at least 32 bits. Where its values are known, from a Range on a field, parameter or loop counter, a length, or arithmetic on them, it is kept in the smallest type that holds them, and worked out in 16 bits when every value fits, as 8-bit machines do fastest; the answer is always the one 32 bits would give. Decimal<2>, imported from Std.Numbers, is a fixed-point number with two decimal places, exact and portable to machines without floating point. A literal such as 2.50 is a Decimal wherever one is expected, and an Int works anywhere a Decimal does. Adding and subtracting are exact; multiplying and dividing round half to even. Bit operators work on Int and bind tighter than comparisons, so `keys & FIRE != 0` tests the bit. Shifts go from 0 to 31. A let can name its type. Bytes holds values from 0 to 255, written as a list of numbers.

```croft
use { Decimal } from Std.Numbers

const FIRE: Int = 1 << 4
const MASK: Int = 0b0001_1111
const SCREEN: Int = 0x4000

fn fire_pressed(keys: Int): Bool = keys & FIRE != 0

fn flip(keys: Int): Int = ~keys & MASK

fn with_tax(price: Decimal<2> where Range(0, 999.99)): Decimal<2> = price * 1.20

// 10.00 / 3 is 3.33; 0.05 / 2 rounds to 0.02, the even neighbour
fn share(total: Decimal<2>, people: Int): Decimal<2> = total / people

let header: Bytes = [0x44, 0x4E, 1]   // length, get, add, slice, and + to join

let price: Decimal<2> = 4
price.to_string()  // "4.00"
price.round()      // 4, an Int
```
