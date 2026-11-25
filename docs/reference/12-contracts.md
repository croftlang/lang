# Contracts

What code needs and what it promises, read before its body. A precondition with an error is a failure the domain expects: always checked, handled by the caller. A plain precondition is a mistake by the caller: checked in debug builds. where constraints limit parameters and results; a parameter's constraints can follow its type or go in the contract, whichever reads better. Invariants hold after every command, and calls lists the ports a service touches. It is separate from use, which only brings in names from another context. Use them to write each domain rule once, where it applies, to name failures in the domain's words, and to keep bodies simple.

```croft
// expected failure: always checked, the caller handles OutOfStock
command Add(item: ItemId, price: Price, in_stock: Int) {
    in_stock > 0 or OutOfStock { item }
} = {
    set items = items.add(item)
    set total = total with { pence: total.pence + price.pence }
}

// caller's mistake: checked in debug builds
fn average(numbers: List<Int>): Int {
    !numbers.is_empty
} = numbers.sum(n -> n) / numbers.length

// constraints on a parameter and on the result
fn heal(amount: Int where Range(1, 50)): Int where Range(0, 100) = ...

// the same constraints, kept out of a long signature
fn attack(damage: Int, target: EnemyId): Result<Room, Room.Error> {
    damage where Range(1, 20)
    target where Alive
} = ...

// true after every command, or the command fails and nothing changes
aggregate Basket {
    invariant Affordable or Overdrawn
}

// the only ports this service can touch
service PlayTurn(room_id: RoomId, pressed: Option<Key>): Result<Unit, GameError> {
    calls Screen, Rooms
} = ...
```
