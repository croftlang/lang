// Breaks one constraint, chosen by the command-line argument, so the test can see
// each kind of check stop a debug build.
context Clinic {
    use { Decimal } from Std.Numbers

    constraint Even(number: Int) = number % 2 == 0

    value Dose {
        amount: Int where Range(1, 50)
    }

    fn heal(amount: Int where Range(1, 50)): Int = amount * 2

    value Score: Int where Range(0, 999)

    // Kept in eight characters of its own, not the size of every string.
    value Patient {
        name: String where Length(1, 8)
    }

    fn pay(amount: Decimal<2> where Range(0.01, 99.99)): Decimal<2> = amount

    fn half(number: Int): Int where Even = number / 2

    // Its constraint is in the contract, which checks the same as writing it inline.
    fn count(numbers: List<Int>): Int {
        numbers where each Even
    } = numbers.length

    service Start(): Unit {
        calls Arguments
    } = {
        let chosen = arguments.all()
        if chosen.contains("argument") {
            let healed = heal(70)
        }
        if chosen.contains("result") {
            let halved = half(6)
        }
        if chosen.contains("field") {
            let dose = Dose { amount: 70 }
        }
        if chosen.contains("each") {
            let counted = count([2, 3])
        }
        if chosen.contains("score") {
            let score = Score(chosen.length * 5000)
        }
        if chosen.contains("pay") {
            let paid = pay(120)
        }
        if chosen.contains("name") {
            let patient = Patient { name: "Bartholomew" }
        }
        if chosen.contains("join") {
            let joined = "a sentence of twenty " + "and another of twenty"
        }
        if chosen.contains("shift") {
            let shifted = 1 << chosen.length * 40
        }
        Unit
    }
}
