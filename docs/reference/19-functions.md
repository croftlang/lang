# Functions

Pure domain services: calculations that don't belong to one value or aggregate. A signature, an optional contract, and a body after =. With no return type it gives back Unit. A return type can carry where constraints, like a field. A comment above documents it.

```croft
// Manhattan distance between two positions.
fn distance(start: Position, end: Position): Int where Range(0, SCREEN_WIDTH + SCREEN_HEIGHT) =
    abs(start.x - end.x) + abs(start.y - end.y)

// The enemy closest to a position, or none if there are no enemies.
fn nearest_enemy(start: Position, enemies: List<Room.Enemy>): Option<EnemyId> = enemies
    .min_by(e -> distance(start, e.position))
    .map(e -> e.id)
```
