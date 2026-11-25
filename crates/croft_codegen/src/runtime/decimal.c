
/* Decimals: whole numbers of hundredths for Decimal<2>, rounded half to even. */

static unsigned long magnitude(croft_int value)
{
    return value < 0 ? 0UL - (unsigned long) value : (unsigned long) value;
}

static croft_int with_sign(unsigned long value, croft_bool negative)
{
    return negative ? -(croft_int) value : (croft_int) value;
}

/* n / d rounded half to even. Compares r with d - r, so r * 2 can't overflow. */
static unsigned long divide_even(unsigned long n, unsigned long d)
{
    unsigned long quotient = n / d;
    unsigned long remainder = n % d;
    if (remainder > d - remainder || (remainder == d - remainder && (quotient & 1UL))) {
        quotient++;
    }
    return quotient;
}

croft_int croft_divide_even(croft_int numerator, croft_int denominator)
{
    CROFT_CHECK(denominator != 0, "division by zero");
    if (denominator == 0) {
        return 0;
    }
    return with_sign(divide_even(magnitude(numerator), magnitude(denominator)),
                     (numerator < 0) != (denominator < 0));
}

/* a * b / scale, splitting a into whole and part so the product stays small. */
croft_int croft_decimal_multiply(croft_int a, croft_int b, croft_int scale)
{
    unsigned long left = magnitude(a);
    unsigned long right = magnitude(b);
    unsigned long whole = (left / (unsigned long) scale) * right;
    unsigned long part = divide_even((left % (unsigned long) scale) * right, (unsigned long) scale);
    return with_sign(whole + part, (a < 0) != (b < 0));
}

/* a * scale / b, as long division: the whole part, then the remainder scaled. */
croft_int croft_decimal_divide(croft_int a, croft_int b, croft_int scale)
{
    unsigned long left;
    unsigned long right;
    CROFT_CHECK(b != 0, "division by zero");
    if (b == 0) {
        return 0;
    }
    left = magnitude(a);
    right = magnitude(b);
    return with_sign((left / right) * (unsigned long) scale
                         + divide_even((left % right) * (unsigned long) scale, right),
                     (a < 0) != (b < 0));
}

void croft_decimal_to_string(croft_int value, croft_int places, char *out, croft_u16 *length, croft_u16 capacity)
{
    unsigned long scale = 1;
    unsigned long remaining;
    croft_int place;
    for (place = 0; place < places; place++) {
        scale *= 10;
    }
    remaining = magnitude(value);
    croft_int_to_string((croft_int) (remaining / scale), out, length, capacity);
    if (value < 0 && *length < capacity - 1) {
        for (place = *length; place > 0; place--) {
            out[place] = out[place - 1];
        }
        out[0] = '-';
        (*length)++;
    }
    if (*length < capacity) {
        out[*length] = '.';
        (*length)++;
    }
    remaining %= scale;
    while (scale > 1 && *length < capacity) {
        scale /= 10;
        out[*length] = (char) ('0' + remaining / scale);
        (*length)++;
        remaining %= scale;
    }
}
