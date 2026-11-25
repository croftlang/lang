# Identities

A distinct type whose only job is to tell entities apart. An EnemyId can never be passed where a RoomId is expected.

An identity is declared in the context and names the aggregate or entity it identifies after `for`. That aggregate or entity holds it in one field, which can have any name. Other fields of an identity type are references: a `Hall` with `next: RoomId` points at a room, and its own identity is still the field of type `HallId`. When two aggregates have an entity of the same name, write which one: `for Room.Enemy`.

```croft
identity RoomId for Room: Int where Range(1, 20)
identity PlayerId for Player: Int where Range(1, 2)
identity EnemyId for Enemy: Int where Range(0, MAX_ENEMIES - 1)
```
