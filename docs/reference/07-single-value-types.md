# Single-value types

A value around one value of another type, so a score can't be mixed up with any other Int. Make one by calling its name, or with a literal where one is expected, and read what it holds with .value. It has none of the operators of the type it holds, so arithmetic makes a new one, which checks its constraints.

```croft
value Score: Int where Range(0, 999_999)
value Name: String where Length(1, 12)

let best: Score = 100
let better = Score(best.value + 10)

fn greet(name: Name): String = "Hello, " + name.value

best + 10      // doesn't compile: a Score has no +
```
