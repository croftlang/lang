// A room: one screen with the player, the enemies in it and the way out.
context Dungeon
aggregate Room

identity id: RoomId
player: Player
enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive
treasure: List<Treasure> where Length(0, 8)
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

invariant ExitClear or ExitBlocked

error NotFound

error EnemyMissing {
    enemy_id: EnemyId
}

// An enemy is standing on the exit; usually a bad room file.
error ExitBlocked

event PlayerMoved { to: Position }
event EnemyDamaged { enemy_id: EnemyId  remaining: Int }
event EnemyDefeated { enemy_id: EnemyId  at: Position }
event TreasureDropped { treasure: Treasure }
