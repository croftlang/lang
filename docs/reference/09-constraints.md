# Constraints

Named, pure checks that return Bool. The first parameter is the value being checked. Constraints attach to fields, to whole values and aggregates as invariants, to parameters, and to each item of a list. They can be generic and can call each other.

```croft
// built in: Length(4) with one argument means exactly 4 long
constraint Range(number: Int, low: Int, high: Int) = number >= low && number <= high
constraint Length(items: String or List<T>, low: Int, high: Int) = items.length >= low && items.length <= high

// your own
constraint Alive(enemy: Room.Enemy) = enemy.hit_points > 0
constraint OnScreen(position: Position) =
    Range(position.x, 0, SCREEN_WIDTH - 1) && Range(position.y, 0, SCREEN_HEIGHT - 1)

// on a field, with arguments
hit_points: Int where Range(0, 100)

// on a list and on each of its items
enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive

// Length also sizes a string: this field holds 12 characters, not 32. So do
// the strings made from it, such as title.trim() or each part of a split.
// Longer text is cut short, and a debug build stops and names the field.
title: String where Length(1, 12)

// on a whole aggregate, relating its fields
constraint ExitClear(room: Room) = room.enemies.all(e -> e.position != room.exit)

aggregate Room {
    invariant ExitClear or ExitBlocked
}

// on a parameter
command Heal(amount: Int where Range(1, 50)) { ... }
```
