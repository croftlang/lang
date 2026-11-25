# Translations

The anti-corruption layer: maps one context's term to another's, so neither leaks into the other.

```croft
// in context Inventory
translation TreasureToItem(treasure: Dungeon.Treasure): Item =
    match treasure.kind {
        Coins(count) -> Item.Gold(amount: count)
        Flask -> Item.Potion(heal: 20)
        Key(door) -> Item.Key(door: door)
    }
```
