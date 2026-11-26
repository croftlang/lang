# Constraints

Named, pure checks that return Bool. The first parameter is the value being checked. Constraints attach to fields, to whole values and aggregates as invariants, to parameters, and to each item of a list. They can be generic and can call each other.

```croft
// built in: Length(4) with one argument means exactly 4 long
constraint Range(number: Int, low: Int, high: Int) = number >= low && number <= high
constraint Length(items: String or List<T>, low: Int, high: Int) = items.length >= low && items.length <= high

// your own
constraint Alive(enemy: Room.Enemy) = enemy.hit_points > 0
constraint OnScreen(position: Position) =
    Range(position.x, 0, SCREEN_WIDTH - 1) && Range(position.y, 0, SCREEN_HEIGHT - 1)

// on a field, with arguments. Range also sizes a number: this one is kept in
// one byte, and arithmetic on it is worked out in 16 bits where that is enough.
hit_points: Int where Range(0, 100)

// on a list and on each of its items
enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive

// Length also sizes a string: this field holds 12 characters, not 32. So do
// the strings made from it, such as title.trim() or each part of a split.
// Longer text is cut short, and the program stops and names the field.
title: String where Length(1, 12)

// on a whole aggregate, relating its fields
constraint ExitClear(room: Room) = room.enemies.all(e -> e.position != room.exit)

aggregate Room {
    invariant ExitClear or ExitBlocked
}

// on a parameter
command Heal(amount: Int where Range(1, 50)) { ... }
```

## When constraints are checked

The compiler works out every constraint it can. Wherever a value is known when the program is compiled, a broken constraint is an error from `croft check`, and editors show it as you type. A value is known when it is a literal, a constant, a list of known values, arithmetic or `.length` on them, a `let` name that nothing in the body changes with `set`, or what a function or translation gives back for known arguments. This holds for arguments to functions, services, commands and factories, for fields in records and `with`, for single values, for enum variants, and for the result of a function.

```croft
constraint Overdose(mg: Int, max: Int) = mg <= max

fn give(mg: Int where Overdose(4000)): Int = mg

// error: `give` is given `5000` for `mg`, which breaks `Overdose(4000)`
let given = give(5000)
```

A value that is only known while the program runs, such as input or something read from a port, is checked when it reaches the constraint, and a broken one stops the program with a message naming it: `give: mg must satisfy Overdose(4000)`. This happens on every target unless its section of `croft.toml` sets `checks = false`.

## Failing with an error

A constraint on a parameter can end with `or` and an error. Breaking it is then not a mistake but a failure the domain expects: the call fails with that error, which the caller handles like any other, so `croft check` accepts a call it knows will fail. It is always checked, whatever the target's `checks` setting. Only a parameter's constraint, inline or in the contract, can have `or`; a field, single value, result or loop counter has nowhere to fail, so its constraints must hold.

```croft
error Overdosed

fn give(mg: Int where Overdose(4000) or Overdosed): Result<Int, Overdosed> = mg

// compiles; fails with Overdosed
let given = give(5000)?
```

