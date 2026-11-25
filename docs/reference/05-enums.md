# Enums

A fixed set of named variants. Variants can carry data, and are written with a dot: Direction.North, TreasureKind.Coins(count: 20).

```croft
enum Direction { North, East, South, West }

enum TreasureKind {
    Coins(count: Int where Range(1, 99))
    Flask
    Key(door: DoorId)
}
```
