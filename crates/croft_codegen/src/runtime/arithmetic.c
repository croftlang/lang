
/* Whole numbers: exactly 32 bits on every machine. Arithmetic past them
   stops the program when checks are on, and wraps around in 32 bits when
   they are off; dividing by zero stops it, or gives 0. The work is done in
   unsigned numbers, where C says what happens past the end, and never in
   signed ones, where it does not. */

/* The number whose 32 bits are the low 32 bits of bits, without relying on
   the compiler to wrap a number too big for a croft_int. */
croft_int croft_int_from_bits(unsigned long bits)
{
    bits &= 0xFFFFFFFFUL;
    return bits > 0x7FFFFFFFUL ? -(croft_int) (0xFFFFFFFFUL - bits) - 1 : (croft_int) bits;
}

/* How far a number is from zero, which for the lowest is 2147483648. */
unsigned long croft_int_magnitude(croft_int value)
{
    return value < 0 ? 0UL - (unsigned long) value : (unsigned long) value;
}

croft_int croft_int_add(croft_int a, croft_int b)
{
    CROFT_CHECK(b < 0 ? a >= CROFT_INT_MIN - b : a <= CROFT_INT_MAX - b, "a whole number doesn't fit in 32 bits");
    return croft_int_from_bits((unsigned long) a + (unsigned long) b);
}

croft_int croft_int_subtract(croft_int a, croft_int b)
{
    CROFT_CHECK(b < 0 ? a <= CROFT_INT_MAX + b : a >= CROFT_INT_MIN + b, "a whole number doesn't fit in 32 bits");
    return croft_int_from_bits((unsigned long) a - (unsigned long) b);
}

/* Whether a * b fits in 32 bits. */
croft_bool croft_int_multiply_fits(croft_int a, croft_int b)
{
    unsigned long left = croft_int_magnitude(a);
    unsigned long right = croft_int_magnitude(b);
    unsigned long most = (a < 0) != (b < 0) ? 0x80000000UL : 0x7FFFFFFFUL;
    return right == 0 || left <= most / right;
}

/* The product's low 32 bits are the same however wide unsigned long is. */
croft_int croft_int_multiply(croft_int a, croft_int b)
{
    CROFT_CHECK(croft_int_multiply_fits(a, b), "a whole number doesn't fit in 32 bits");
    return croft_int_from_bits((unsigned long) a * (unsigned long) b);
}

/* Rounds toward zero, which C89 leaves up to the compiler for a number
   below zero. The lowest number divided by -1 is one past the highest. */
croft_int croft_int_divide(croft_int a, croft_int b)
{
    unsigned long quotient;
    CROFT_CHECK(b != 0, "division by zero");
    CROFT_CHECK(a != CROFT_INT_MIN || b != -1, "a whole number doesn't fit in 32 bits");
    if (b == 0) {
        return 0;
    }
    quotient = croft_int_magnitude(a) / croft_int_magnitude(b);
    return croft_int_from_bits((a < 0) != (b < 0) ? 0UL - quotient : quotient);
}

/* Has the sign of a, as the quotient rounds toward zero. */
croft_int croft_int_remainder(croft_int a, croft_int b)
{
    unsigned long remainder;
    CROFT_CHECK(b != 0, "division by zero");
    if (b == 0) {
        return 0;
    }
    remainder = croft_int_magnitude(a) % croft_int_magnitude(b);
    return croft_int_from_bits(a < 0 ? 0UL - remainder : remainder);
}

croft_int croft_int_negate(croft_int a)
{
    CROFT_CHECK(a != CROFT_INT_MIN, "a whole number doesn't fit in 32 bits");
    return croft_int_from_bits(0UL - (unsigned long) a);
}

croft_int croft_int_abs(croft_int a)
{
    CROFT_CHECK(a != CROFT_INT_MIN, "a whole number doesn't fit in 32 bits");
    return croft_int_from_bits(croft_int_magnitude(a));
}

/* The base multiplied by itself exponent times, through croft_int_multiply,
   so a power past 32 bits stops the program or wraps around as `*` does.
   An exponent below zero stops it, or gives 0. The base is only squared
   when more of the exponent needs it, so a power that fits never stops. */
croft_int croft_int_pow(croft_int base, croft_int exponent)
{
    croft_int power = 1;
    CROFT_CHECK(exponent >= 0, "pow's exponent is below zero");
    if (exponent < 0) {
        return 0;
    }
    while (exponent > 0) {
        if (exponent % 2 == 1) {
            power = croft_int_multiply(power, base);
        }
        exponent /= 2;
        if (exponent > 0) {
            base = croft_int_multiply(base, base);
        }
    }
    return power;
}

/* The number kept from low to high. A low above the high stops the
   program, or gives the low. Decimals with the same places compare as
   their whole numbers of smallest units, so they are clamped here too. */
croft_int croft_int_clamp(croft_int number, croft_int low, croft_int high)
{
    CROFT_CHECK(low <= high, "clamp's low is above its high");
    if (number > high) {
        number = high;
    }
    return number < low ? low : number;
}

/* croft_int_clamp for a Float. */
double croft_float_clamp(double number, double low, double high)
{
    CROFT_CHECK(low <= high, "clamp's low is above its high");
    if (number > high) {
        number = high;
    }
    return number < low ? low : number;
}

/* The checked arithmetic of Std.Numbers: whether the answer fits in 32
   bits, with the answer written to out when it does and 0 when it doesn't.
   Whatever the checks, these never stop the program. */
croft_bool croft_int_checked_add(croft_int a, croft_int b, croft_int *out)
{
    croft_bool fits = b < 0 ? a >= CROFT_INT_MIN - b : a <= CROFT_INT_MAX - b;
    *out = fits ? croft_int_from_bits((unsigned long) a + (unsigned long) b) : 0;
    return fits;
}

croft_bool croft_int_checked_subtract(croft_int a, croft_int b, croft_int *out)
{
    croft_bool fits = b < 0 ? a <= CROFT_INT_MAX + b : a >= CROFT_INT_MIN + b;
    *out = fits ? croft_int_from_bits((unsigned long) a - (unsigned long) b) : 0;
    return fits;
}

croft_bool croft_int_checked_multiply(croft_int a, croft_int b, croft_int *out)
{
    croft_bool fits = croft_int_multiply_fits(a, b);
    *out = fits ? croft_int_from_bits((unsigned long) a * (unsigned long) b) : 0;
    return fits;
}

/* Through unsigned long, since C leaves shifting a negative number left
   undefined; the bits shifted past the 32 are lost. */
croft_int croft_int_shift_left(croft_int value, croft_int amount)
{
    return croft_int_from_bits((unsigned long) value << amount);
}
