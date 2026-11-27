# Built-ins

Functions and methods every program has, with no import. A list of entities is also looked up, replaced and removed by identity. Methods that take a lambda call it with each item; `fold`'s lambda takes two parameters, the total so far and the item, and gives back the next total, which starts as the first argument. `sort` puts numbers in order by size and strings byte by byte; `sort_by` orders by the whole number its lambda gives. `unique` keeps the first of each item, in order. `insert` puts an item at a place from 0 to the list's length, moving the items from there on one place later; a place known to be below 0 is a `croft check` error, and one past either end stops the program, unless the target sets `checks = false`, when the item goes at the nearest end.

```croft
// functions
Some(value)   None   Ok(value)   Err(error)
Range(n, low, high)   Length(items, low, high)

// strings and numbers
name.length   name.is_empty   "dun" + "geon"   score.to_string()

// lists
items.length   items.is_empty   items.first   items.last   // first, last: Option
items.add(item)   items.drop(1)   items.take(3)   items.get(0)   items.contains(item)
items.index_of(item)   items.reverse()   items + more   [first] + rest
items.unique()   items.insert(0, item)   // [3, 1, 3].unique() is [3, 1]
items.filter(i -> i.ready)   items.map(i -> i.name)   items.find(i -> i.ready)
items.all(i -> i.ready)   items.any(i -> i.ready)   items.count(i -> i.ready)
items.sum(i -> i.points)   items.min_by(i -> i.cost)   items.max_by(i -> i.cost)
scores.sort()   names.sort()   items.sort_by(i -> i.cost)   // same keys keep their order
items.fold(0, (total, i) -> total + i.points)   // a total from each item in turn

// lists of entities, by identity
enemies.get(enemy_id)   enemies.contains(enemy_id)
enemies.replace(enemy)   enemies.remove(enemy_id)

// options
found.is_some   found.is_none   found.map(e -> e.id)   found.unwrap_or(0)

// results
loaded.is_ok   loaded.is_err   loaded.map(r -> r.name)   loaded.unwrap_or(fallback)
loaded.map_err(e -> GameError.SaveDamaged)
```
