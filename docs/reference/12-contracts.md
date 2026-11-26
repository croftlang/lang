# Contracts

What code needs and what it promises, read before its body. Its constraints can follow a parameter's type or go in the contract, whichever reads better, and a line of the contract such as `from != to` is a constraint without a name. A constraint without `or` is a mistake by the caller if broken: an error from `croft check` when the values it reads are known, and otherwise a stop while the program runs, unless the target sets `checks = false`. A constraint with `or` is a failure the domain expects: always checked, it fails the call with its error, which the caller handles. Only a parameter's constraint or a contract line can have `or`; a field or result has nowhere to fail, so its constraints must hold. Invariants hold after every factory and command, and calls lists the ports a service touches. It is separate from use, which only brings in names from another context. Use them to write each domain rule once, where it applies, to name failures in the domain's words, and to keep bodies simple.

```croft
// expected failure: always checked, the caller handles OutOfStock
command Add(item: ItemId, price: Price, in_stock: Int) {
    in_stock > 0 or OutOfStock { item }
} = {
    set items = items.add(item)
    set total = total with { pence: total.pence + price.pence }
}

// caller's mistake: `average([])` doesn't compile; an unknown list is checked as it runs
fn average(numbers: List<Int>): Int {
    !numbers.is_empty
} = numbers.sum(n -> n) / numbers.length

// constraints on a parameter and on the result
fn heal(amount: Int where Range(1, 50)): Int where Range(0, 100) = ...

// expected failure on a parameter: `give(5000)` compiles, and fails with Overdosed
fn give(mg: Int where Overdose(4000) or Overdosed): Result<Int, Overdosed> = mg

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
