# Body expressions

Everything a body can contain. Lists are written in brackets: [a, b], or [] when empty, and so are sets where a set is expected; maps as [key: value], or [:] when empty. A lambda is a parameter, `->` and a value: `e -> e.id`. One with two or more parameters puts them in brackets: `(total, item) -> total + item`. A single parameter never takes brackets: `(e) -> e.id` is an error. A last match arm of _ handles every variant the arms above it don't. There is no else: an if is a guard that returns its value when the condition holds, and the last line of a body is the result when no guard matched.

```croft
let gap = distance(player.position, enemy.position)

if hit_points == 0 {
    Defeated
}

Fighting

match kind {
    Coins(count) -> count * 10
    Flask -> 50
    Key(door) -> 0
}

match kind {
    Coins(count) -> count
    _ -> 0          // every variant not handled above
}

enemies.filter(e -> e.hit_points < 10).map(e -> e.position)

items.sum(item -> points_for(item))

items.fold(0, (total, item) -> total + points_for(item))

player with { lives: player.lives - 1 }

let corners = [Position { x: 0, y: 0 }, Position { x: 248, y: 184 }]

let room = rooms.find(room_id)?
```
