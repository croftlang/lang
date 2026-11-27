# Project file

croft.toml sits beside the .croft files, like Cargo.toml in Rust. It holds what is about the build, not the domain: for each target, the adapter that fills each port, the C files holding its native functions, and optionally a main service to run instead of the domain, such as a test entry point. Services only ask for ports; they never name adapters. The compiler checks every name here against the program.

A project doesn't need one. Without croft.toml, the program runs its domain on one `desktop` target, with checks on and no adapters, and is named after the domain in snake_case: `domain Logistics` builds `logistics`. A folder with no domain has nothing to run, but can still be checked. When croft.toml has only one target, `croft compile`, `croft run` and `croft size` use it without `--target`. Given a single file, such as `croft run main.croft`, they use the croft.toml in that file's folder.

Every target checks, while the program runs, the constraints without `or` that the compiler could not work out, and stops with a message when one is broken. `checks = false` turns that off for a target, saving the code and time the checks take on a small machine; a broken rule then goes unnoticed, a number too big for its `Range` is cut down to fit, arithmetic past 32 bits wraps around, and dividing by zero gives 0. Constraints with `or` are part of what the program does, so they are checked either way.

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

`native` lists the project's own C files for a target. `croft compile` copies them beside the generated C, and `croft run` builds everything with the compiler named by `CC`, or `cc`, as optimised strict C89: `-std=c89 -pedantic -Wall -Wextra -Wdeclaration-after-statement -O2`. Every warning is an error in the C Croft writes, standard adapters included; in native files the warnings are shown but don't stop the build.
