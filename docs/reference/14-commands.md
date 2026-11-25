# Commands

The only way to change an aggregate; a new one is made by a [factory](13-factories.md). A command is declared inside it, can have a contract and has a body. It raises events with emit; the compiler works out which ones from the body. `set` describes the new state. Nothing changes unless the whole command succeeds.

```croft
// Damages one enemy, and defeats it at zero hit points.
command Attack(enemy_id: EnemyId, damage: Int where Range(1, 20)) {
    enemies.contains(enemy_id) or EnemyMissing { enemy_id }
} = {
    let enemy = enemies.get(enemy_id)
    let remaining = max(0, enemy.hit_points - damage)
    set enemies = enemies.replace(enemy with { hit_points: remaining })
    emit EnemyDamaged { enemy_id, remaining }
    if remaining == 0 {
        emit EnemyDefeated { enemy_id, at: enemy.position }
    }
}
```
