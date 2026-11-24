# Glossary

The shared language for sp. These terms are used the same way in the
language, the compiler's code, error messages and conversation. If a new
concept needs a name, add it here first.

## Language terms

### Struct
A named group of typed fields. Structs describe the *shape* of data, such
as a function's input or output. Constraints can be attached to each field
and to the struct as a whole.

```sp
struct Money {
    amount: Int is Positive
    currency: Currency
}
```

Compiles to a Rust `struct` plus an implementation of `Validate`.

### Field
One named, typed value inside a struct, such as `amount: Int`. Field names
are `snake_case`.

### Enum
A type with a fixed set of named variants and no data attached to them.
Variants are referred to as `Currency.Gbp`.

```sp
enum Currency { Gbp, Usd, Eur }
```

### Const
A named value fixed at compile time. Constants are `Int`, `Float`, `Bool`
or `String` and are named in `UPPER_SNAKE_CASE`.

```sp
const MAX_LINES: Int = 100
```

### Constraint
A named rule that a value must satisfy. A constraint is a pure expression
that returns `Bool`. It has no side effects and no loops, so the compiler
can always check it. The first parameter is the value being checked. Any
further parameters are filled in where the constraint is used.

```sp
constraint MaxLength(text: String, limit: Int) = text.length <= limit
constraint NonEmpty<T>(list: List<T>) = list.length > 0
```

Constraints are where the logic of a program lives. A rule is written
once and can be reused across many structs and functions.

### Field constraint
A constraint attached to a field with `is`. The field is passed as the
constraint's first parameter.

```sp
description: String is NotBlank, MaxLength(200)
```

### Struct constraint
A constraint attached to a whole struct, written after its closing brace.
Used for rules that relate fields to each other.

```sp
struct Invoice { ... } is DueAfterIssue
```

### Function
A named transformation from inputs to an output, described by its contract.
The developer writes the signature and contract in sp. The body is not
written in sp: it is generated, and checked against the contract.

```sp
fn scale_by_ten(value: Int) -> Int {
    requires value <= 1_000_000 && value >= -1_000_000
    ensures output == value * 10
    description "Multiplies a value by ten."
}
```

### Signature
A function's name, parameters and output type. Port operations have
signatures too.

### Contract
Everything a function promises, apart from its signature: its `uses`,
`requires`, `ensures` and `description`.

### Requires
A precondition: a `Bool` expression over the function's inputs that must
hold when the function is called. It is the caller's responsibility.

### Ensures
A postcondition: a `Bool` expression over the inputs and `output` that
must hold when the function returns. It is the body's responsibility.
The more completely `ensures` describes the output, the less room any
generated body has to behave differently.

### Output
The name `ensures` uses for the value a function returns. It cannot be
used as a parameter name.

### Description
A plain-language statement of what a function does. Every function needs
one. It documents the function and guides body generation, but the
compiler cannot check it, so `requires` and `ensures` should capture as
much of it as possible.

### Uses
The list of ports a function needs. A function can only reach the outside
world through the ports it names here. Each port becomes an extra
parameter of the generated function.

### Port
An interface to something outside the program's domain logic, such as a
database, an HTTP API or a clock. A port lists operations by signature
only. It says *what* the domain needs, never *how* it is done.

```sp
port InvoiceRepository {
    fn find(id: InvoiceId) -> Option<Invoice>
    fn save(invoice: Invoice) -> Result<Unit, StorageError>
}
```

Compiles to a Rust trait.

### Operation
One function signature inside a port, such as `find` above.

### Adapter
A concrete implementation of a port, such as a Postgres repository or an
in-memory fake for tests. Adapters keep side effects at the edges, so the
domain logic stays pure and easy to check.

```sp
adapter InMemoryInvoices for InvoiceRepository
```

Compiles to a Rust struct implementing the port's trait.

### Body
The implementation of a function or adapter operation. Bodies are never
written in sp. For now they are generated as `todo!()` placeholders.

## Types

| sp | Rust | Notes |
|---|---|---|
| `Int` | `i64` | |
| `Float` | `f64` | |
| `Bool` | `bool` | |
| `String` | `String` | `.length` counts characters |
| `Unit` | `()` | No meaningful value |
| `List<T>` | `Vec<T>` | `.length` |
| `Option<T>` | `Option<T>` | `.is_some`, `.is_none` |
| `Result<T, E>` | `Result<T, E>` | `.is_ok`, `.is_err` |

### Type parameter
A placeholder type in a generic constraint, such as `T` in `NonEmpty<T>`.

## Compiler terms

### Project
A folder of `.sp` files compiled together as one program, or a single
`.sp` file.

### Check
Reading, parsing and checking a project without generating code
(`sp check`). This covers names, types, constraint usage and naming
conventions.

### Compile
Checking a project, generating a Rust crate from it in `target/sp/` and
building that crate with cargo (`sp compile`).

### Generated crate
The Rust crate `sp compile` writes. It is output, not source: change the
`.sp` files instead of editing it.

### Validate
The Rust trait every generated struct implements. `validate()` checks a
value against the constraints on its fields, its nested structs and the
struct itself, and returns the first `ConstraintViolation` it finds.

### Constraint violation
The error returned when a value breaks a constraint. It names the
constraint and where it was attached, such as `Invoice.lines`.

### Contract check
The checks a generated function runs around its body in debug builds:
input validation and `requires` before the body, `ensures` after it.

### Diagnostic
An error the compiler reports, with its file, line and column.

## Planned terms

These are not built yet but have agreed names.

### Body generation
Producing a function's body with AI from its signature, contract and the
constraints of its types, then checking the result against the contract.

### Lockfile
A committed record of generated bodies, so the same source always builds
the same program. A body is only generated again when its function's
contract changes.

### Escape hatch
A hand-written body for a function or adapter operation, used where
generation is not good enough. It is checked against the contract like a
generated one.
