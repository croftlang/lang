# Issuing commands

Commands are issued, never called. `on` names which aggregate: a value you hold gives you the new aggregate back, and an identity in brackets loads, changes and saves it in one step. Arguments are always named, and a command with no parameters drops its brackets.

```croft
const MAIN_SCOREBOARD: ScoreboardId = 1

// on a value you already hold
let attacked = issue Attack(enemy_id: target, damage: 5) on room?
rooms.save(attacked)?

// on an identity: loaded, changed and saved for you
issue AddPoints(amount: 100) on Scoreboard(MAIN_SCOREBOARD)?

// no parameters, no brackets
issue Reset on Scoreboard(MAIN_SCOREBOARD)?
```
