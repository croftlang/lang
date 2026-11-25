// The rooms the player explores, one screen at a time.
context Dungeon

publish {
    SetUpRooms,
    PlayTurn,
    FIRST_ROOM,
    GAME_TITLE,
    RoomId,
    EnemyId,
    Position,
    Key,
    Screen,
    Keyboard,
    Room.EnemyDefeated
}

const SCREEN_WIDTH: Int = 256
const SCREEN_HEIGHT: Int = 192
const MAX_ENEMIES: Int = 16
const GAME_TITLE: String = "Dungeon"
const FIRST_ROOM: RoomId = 1
const PLAYER_SPRITE: SpriteId = 0

identity RoomId: Int where Range(1, 20)
identity PlayerId: Int where Range(1, 2)
identity EnemyId: Int where Range(0, MAX_ENEMIES - 1)
identity DoorId: Int where Range(1, 40)
identity SpriteId: Int where Range(0, 63)

enum Direction { North, East, South, West }

enum Key { Up, Down, Left, Right, Fire, Quit }

enum TreasureKind {
    Coins(count: Int where Range(1, 99))
    Flask
    Key(door: DoorId)
}

// Values live at context level and are shared.
value Position {
    x: Int where Range(0, SCREEN_WIDTH - 1)
    y: Int where Range(0, SCREEN_HEIGHT - 1)
}

value Treasure {
    kind: TreasureKind
    at: Position
}

constraint Alive(enemy: Room.Enemy) = enemy.hit_points > 0

constraint ExitClear(room: Room) = room.enemies.all(e -> e.position != room.exit)
