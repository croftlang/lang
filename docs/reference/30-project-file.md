# Project file

croft.toml sits beside the .croft files, like Cargo.toml in Rust. It holds what is about the build, not the domain: for each target, the adapter that fills each port, the C files holding its native functions, and optionally a main service to run instead of the domain, such as a test entry point. Services only ask for ports; they never name adapters. The compiler checks every name here against the program.

Every target checks, while the program runs, the constraints without `or` that the compiler could not work out, and stops with a message when one is broken. `checks = false` turns that off for a target, saving the code and time the checks take on a small machine; a broken rule then goes unnoticed, and a number too big for its `Range` is cut down to fit. Constraints with `or` are part of what the program does, so they are checked either way.

```croft
[package]
name = "dungeon"
version = "0.1.0"

[target.spectrum_48k]
native = ["platform/spectrum.c"]

[target.spectrum_48k.adapters]
Screen = "SpectrumScreen"
Keyboard = "SpectrumKeyboard"
Files = "DivmmcFiles"
Rooms = "FileStorage"

[target.test]
main = "RunTests"

# The same program, measured and trusted, without its runtime checks.
[target.spectrum_48k_unchecked]
checks = false

[target.test.adapters]
Screen = "NullScreen"
Keyboard = "ScriptedKeyboard"
Rooms = "MemoryStorage"
```
