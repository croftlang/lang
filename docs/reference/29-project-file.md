# Project file

dn.toml sits beside the .dn files, like Cargo.toml in Rust. It holds what is about the build, not the domain: for each target, the adapter that fills each port, the C files holding its native functions, and optionally a main service to run instead of the domain, such as a test entry point. Services only ask for ports; they never name adapters. The compiler checks every name here against the program.

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

[target.test.adapters]
Screen = "NullScreen"
Keyboard = "ScriptedKeyboard"
Rooms = "MemoryStorage"
```
