# Built-ins

Functions and methods every program has, with no import. A list of entities is also looked up, replaced and removed by identity. Methods that take a lambda call it with each item. `sort` puts numbers in order by size and strings byte by byte; `sort_by` orders by the whole number its lambda gives.

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
items.filter(i -> i.ready)   items.map(i -> i.name)   items.find(i -> i.ready)
items.all(i -> i.ready)   items.any(i -> i.ready)   items.count(i -> i.ready)
items.sum(i -> i.points)   items.min_by(i -> i.cost)   items.max_by(i -> i.cost)
scores.sort()   names.sort()   items.sort_by(i -> i.cost)   // same keys keep their order

// lists of entities, by identity
enemies.get(enemy_id)   enemies.contains(enemy_id)
enemies.replace(enemy)   enemies.remove(enemy_id)

// options
found.is_some   found.is_none   found.map(e -> e.id)   found.unwrap_or(0)
```
