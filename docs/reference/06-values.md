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
