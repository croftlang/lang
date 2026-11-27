# The smallest program

One context with one function, and a domain that calls it and prints the answer through the built-in Console port. Beside it, dn.toml needs only the program's name and a target.

```croft
context Hello {
    publish { add }

    fn add(a: Int, b: Int): Int = a + b
}

domain Program {
    contexts { Hello }
    use { add } from Hello
    calls { Console }
} = Console.print(add(2, 3).to_string())   // prints 5

// dn.toml
// [package]
// name = "hello"
//
// [target.desktop]
```
