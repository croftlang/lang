# Identities

A distinct type whose only job is to tell entities apart. An EnemyId can never be passed where a RoomId is expected.

```croft
identity RoomId: Int where Range(1, 20)
identity PlayerId: Int where Range(1, 2)
identity EnemyId: Int where Range(0, MAX_ENEMIES - 1)
```
