# Policies

A reaction to an event that issues a command, often on another aggregate or in another context. This is how changes spread without one command touching two aggregates.

```croft
policy DropLootOnDefeat when Room.EnemyDefeated {
    issue DropTreasure(at: event.at) on Room(event.source)
}

// in context Scoring
policy ScoreOnDefeat when Dungeon.Room.EnemyDefeated {
    issue AddPoints(amount: 100) on Scoreboard(MAIN_SCOREBOARD)
}
```
