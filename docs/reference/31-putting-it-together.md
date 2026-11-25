# Putting it together

The whole of the Dungeon context's room.dn file, as one piece.

```croft
// The room the player explores, one screen at a time.
context Dungeon {
    publish {
        RoomId,
        EnemyId,
        Position,
        Room.EnemyDefeated
    }

    const SCREEN_WIDTH: Int = 256
    const SCREEN_HEIGHT: Int = 192
    const MAX_ENEMIES: Int = 16

    identity RoomId: Int where Range(1, 20)
    identity PlayerId: Int where Range(1, 2)
    identity EnemyId: Int where Range(0, MAX_ENEMIES - 1)

    enum Direction { North, East, South, West }

    // Values live at context level and are shared.
    value Position {
        x: Int where Range(0, SCREEN_WIDTH - 1)
        y: Int where Range(0, SCREEN_HEIGHT - 1)
    }

    aggregate Room {
        identity id: RoomId
        player: Player
        enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive
        exit: Position

        // Entities live only inside their aggregate.
        entity Player {
            identity id: PlayerId
            position: Position
            lives: Int where Range(0, 9)
        }

        entity Enemy {
            identity id: EnemyId
            position: Position
            hit_points: Int where Range(0, 100)
        }

        event PlayerMoved { to: Position }
        event EnemyDamaged { enemy_id: EnemyId  remaining: Int }
        event EnemyDefeated { enemy_id: EnemyId  at: Position }

        // Moves the player one step, unless a wall is in the way.
        command Move(direction: Direction) = {
            let next = player.position.step(direction)
            if walkable(next) {
                set player = player with { position: next }
                emit PlayerMoved { to: next }
            }
        }

        // Damages one enemy, and defeats it at zero hit points.
        command Attack(enemy_id: EnemyId, damage: Int where Range(1, 20)) {
            enemies.contains(enemy_id)
        } = {
            let enemy = enemies.get(enemy_id)
            let remaining = max(0, enemy.hit_points - damage)
            set enemies = enemies.replace(enemy with { hit_points: remaining })
            emit EnemyDamaged { enemy_id, remaining }
            if remaining == 0 {
                emit EnemyDefeated { enemy_id, at: enemy.position }
            }
        }
    }
}
```
