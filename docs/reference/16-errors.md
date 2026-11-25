# Errors

An aggregate declares its own errors: a name, optionally with fields describing the failure. A comment directly above an error documents it. The compiler numbers errors automatically. Messages shown to the player are not part of the domain: they live in the outer layer.

```croft
aggregate Room {
    error NotFound

    error EnemyMissing {
        enemy_id: EnemyId
    }

    // Room data has two enemies on one square; usually a bad room file.
    error EnemiesOverlap
}

// naming an error that has fields
enemies.contains(enemy_id) or EnemyMissing { enemy_id }

// player-facing text, in the outer layer
fn message(error: GameError): String = match error {
    EnemyMissing(enemy_id) -> "That enemy has gone."
    NotFound -> "Room missing."
}
```
