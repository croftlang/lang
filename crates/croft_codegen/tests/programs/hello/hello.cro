// The smallest program that shows something: one function, and a domain that
// prints what it gives back through the built-in Console port.
context Hello {
    publish { add }

    fn add(a: Int, b: Int): Int = a + b
}

domain Program {
    contexts { Hello }
    use { add } from Hello
    calls Console
} = console.print(add(2, 3).to_string())
