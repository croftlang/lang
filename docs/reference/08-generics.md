# Generics

Values and functions written once for many types. Type parameters are single capital letters. Each use with different types is an instance, such as Pair<Int, String>, which the compiler makes and checks as if it were written out by hand, and compiles to C of its own. A call works out its type parameters from what it is given and from the type it must give back.

```croft
value Pair<A, B> {
    first: A
    second: B
}

fn swap<A, B>(pair: Pair<A, B>): Pair<B, A> = Pair { first: pair.second, second: pair.first }

fn first_or<T>(items: List<T>, fallback: T): T = items.first.unwrap_or(fallback)

let numbers = Pair { first: 1, second: "one" }   // a Pair<Int, String>
let swapped = swap(numbers)                      // a Pair<String, Int>
let ids: List<RoomId> = [first_or(rooms, FIRST_ROOM)]
```
