// The commands that change a room.
context Dungeon
aggregate Room

// Moves the player one step, unless a wall is in the way.
command Move(direction: Direction) = {
    let next = step(player.position, direction)
    if walkable(next) {
        set player = player with { position: next }
        emit PlayerMoved { to: next }
    }
}

// Damages one enemy, and defeats it at zero hit points.
command Attack(enemy_id: EnemyId, damage: Int where Range(1, 20)) {
    enemies.contains(enemy_id) or EnemyMissing { enemy_id }
} = {
    let enemy = enemies.get(enemy_id)
    let remaining = max(0, enemy.hit_points - damage)
    set enemies = enemies.replace(enemy with { hit_points: remaining })
    emit EnemyDamaged { enemy_id, remaining }
    if remaining == 0 {
        set enemies = enemies.remove(enemy_id)
        emit EnemyDefeated { enemy_id, at: enemy.position }
    }
}

// Leaves a flask where an enemy fell.
command DropTreasure(at: Position) = {
    let dropped = Treasure { kind: TreasureKind.Flask, at }
    set treasure = treasure.add(dropped)
    emit TreasureDropped { treasure: dropped }
}
