// A domain whose body fails, so the program stops with the error's name.
context Shop {
    publish { Open }

    error Closed

    service Open(): Result<Unit, Closed> = Err(Closed)
}

domain Market {
    contexts { Shop }
    use { Open } from Shop
} = {
    Open()?
}
