# Values

Value objects: defined only by their fields, equal when their fields are equal, and never changed. Values can be declared at context level and shared. Make one with its type name and fields; change one by copying it with `with`.

```croft
value Position {
    x: Int where Range(0, SCREEN_WIDTH - 1)
    y: Int where Range(0, SCREEN_HEIGHT - 1)
}

value Treasure {
    kind: TreasureKind
    at: Position
}

let start = Position { x: 10, y: 20 }        // make a new Position
let moved = start with { x: start.x + 1 }    // copy it, changing x
```

A value can have invariants, [constraints](09-constraints.md) on it as a whole that relate its fields. Every value made, by its type name or with `with`, must keep them: making a value can't fail, so an invariant on a value has no `or`. One the compiler knows is broken is an error from `croft check`; otherwise, with checks on, a broken one stops the program with a message naming it, `Span must satisfy Ordered`. Storage checks them on each value it reads back.

```croft
constraint Ordered(span: Span) = span.low <= span.high

value Span {
    low: Int
    high: Int

    invariant Ordered
}

let wide = Span { low: 1, high: 9 }
// error: `Span { low: 9, high: 1 }` breaks `Span`'s invariant `Ordered`
let backwards = Span { low: 9, high: 1 }
```
