# Events

Something that happened, named in the past tense and declared inside the aggregate that raises it. Every event carries its aggregate's identity as event.source. From outside it is Room.EnemyDefeated. An event only counts once its aggregate is saved; if the changed aggregate is never saved, the event is dropped.

```croft
aggregate Room {
    event PlayerMoved { to: Position }
    event EnemyDamaged { enemy_id: EnemyId  remaining: Int }
    event EnemyDefeated { enemy_id: EnemyId  at: Position }
}
```
