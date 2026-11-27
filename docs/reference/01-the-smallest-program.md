# The smallest program

One context with one function, and a domain that calls it and prints the answer through the built-in Console port. It needs no croft.toml: `croft run hello.croft` runs it on the `desktop` target.

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
```
