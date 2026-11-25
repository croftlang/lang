# Adapters

A port implemented in dn or with native C, usually in a Platform context. dn.toml chooses which targets use it, so one adapter can serve several. An adapter's body comes after =, like a function's. It provides every operation its port declares; the types come from the port. Only adapters may call native C functions, and they may hold state changed with set. Every adapter field has a starting value, written after its type and constraints. A native function gets numbers by value, each string as a `croft_string_view` (its characters and length, borrowed for the call), anything else by pointer, and a pointer for its result last.

```croft
// platform/spectrum.dn
context Platform {
    use { Screen, Keyboard, Key } from Dungeon

    adapter SpectrumScreen for Screen = {
        fn clear() = native zx_cls()
        fn draw_sprite(sprite, at) = native sprite_draw(sprite, at.x, at.y)
        fn draw_text(text, at) = native zx_print_at(at.y / 8, at.x / 8, text)
    }

    adapter SpectrumKeyboard for Keyboard = {
        fn pressed() = native read_key()
    }

    // platform/spectrum.c
    // void zx_print_at(croft_int row, croft_int column, croft_string_view text, croft_unit *out);
}

// an adapter written in dn, for tests
adapter ScriptedKeyboard for Keyboard = {
    keys: List<Key> where Length(0, 32) = [Right, Right, Fire]

    fn pressed() = match keys.first {
        Some(next) -> {
            set keys = keys.drop(1)
            Some(next)
        }
        None -> Some(Quit)
    }
}
```
