
/* Decimals: whole numbers of hundredths for Decimal<2>, rounded half to even.
   Each is kept in the 32 bits of a croft_int, so an answer past them stops
   the program when checks are on, and wraps around when they are off. */

/* value, a magnitude, with the sign given, when it fits in 32 bits and over
   is not set; otherwise its low 32 bits, once the check has stopped a
   checked program. */
static croft_int with_sign(unsigned long value, croft_bool negative, croft_bool over)
{
    CROFT_CHECK(!over && value <= (negative ? 0x80000000UL : 0x7FFFFFFFUL), CROFT_DECIMAL_TOO_BIG);
    return croft_int_from_bits(negative ? 0UL - value : value);
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

/* x * y / d rounded half to even, for magnitudes below 2 to the 32 and a d
   from 1 to 2 to the 31. No step needs more than 32 bits: the product is
   made as eight bytes, then divided a bit at a time. *over is set when the
   answer needs more than 32 bits, and its low 32 bits are given back. */
static unsigned long multiply_divide(unsigned long x, unsigned long y, unsigned long d, croft_bool *over)
{
    unsigned char product[8];
    unsigned long carry;
    unsigned long remainder = 0;
    unsigned long quotient = 0;
    int row;
    int column;
    int bit;
    *over = 0;
    for (row = 0; row < 8; row++) {
        product[row] = 0;
    }
    for (row = 0; row < 4; row++) {
        carry = 0;
        for (column = 0; column < 4; column++) {
            carry += product[row + column] + ((x >> (8 * row)) & 0xFFUL) * ((y >> (8 * column)) & 0xFFUL);
            product[row + column] = (unsigned char) (carry & 0xFFUL);
            carry >>= 8;
        }
        product[row + 4] = (unsigned char) carry;
    }
    for (bit = 63; bit >= 0; bit--) {
        remainder = (remainder << 1) | ((product[bit / 8] >> (bit % 8)) & 1U);
        if (quotient & 0x80000000UL) {
            *over = 1;
        }
        quotient = (quotient << 1) & 0xFFFFFFFFUL;
        if (remainder >= d) {
            remainder -= d;
            quotient |= 1UL;
        }
    }
    if (remainder > d - remainder || (remainder == d - remainder && (quotient & 1UL))) {
        if (quotient == 0xFFFFFFFFUL) {
            *over = 1;
        }
        quotient = (quotient + 1UL) & 0xFFFFFFFFUL;
    }
    return quotient;
}

croft_int croft_divide_even(croft_int numerator, croft_int denominator)
{
    CROFT_CHECK(denominator != 0, "division by zero");
    if (denominator == 0) {
        return 0;
    }
    return with_sign(divide_even(croft_int_magnitude(numerator), croft_int_magnitude(denominator)),
                     (numerator < 0) != (denominator < 0), 0);
}

/* a * b / scale. */
croft_int croft_decimal_multiply(croft_int a, croft_int b, croft_int scale)
{
    croft_bool over;
    unsigned long product =
        multiply_divide(croft_int_magnitude(a), croft_int_magnitude(b), (unsigned long) scale, &over);
    return with_sign(product, (a < 0) != (b < 0), over);
}

/* a * scale / b. */
croft_int croft_decimal_divide(croft_int a, croft_int b, croft_int scale)
{
    croft_bool over;
    unsigned long quotient;
    CROFT_CHECK(b != 0, "division by zero");
    if (b == 0) {
        return 0;
    }
    quotient = multiply_divide(croft_int_magnitude(a), (unsigned long) scale, croft_int_magnitude(b), &over);
    return with_sign(quotient, (a < 0) != (b < 0), over);
}

/* A decimal written with an optional "-", digits, and optionally a point
   and up to places more digits, as a whole number of its smallest units.
   Text that isn't one, has more places, or doesn't fit in 32 bits gives 0. */
croft_bool croft_string_to_decimal(croft_string_view text, croft_int places, croft_int *out)
{
    unsigned long value = 0;
    unsigned long limit;
    unsigned long digit;
    croft_int given = 0;
    croft_u16 index = 0;
    croft_u16 whole = 0;
    croft_bool point = 0;
    croft_bool negative = text.length > 0 && text.text[0] == '-';
    if (negative) {
        index = 1;
    }

    /* As croft_string_to_int does, the units are read as an unsigned long
       and refused as soon as they go past what 32 bits hold. */
    limit = negative ? 2147483648UL : 2147483647UL;
    for (; index < text.length; index++) {
        if (text.text[index] == '.' && !point) {
            point = 1;
            continue;
        }
        if (text.text[index] < '0' || text.text[index] > '9') {
            return 0;
        }
        if (point) {
            given++;
        } else {
            whole++;
        }
        if (given > places) {
            return 0;
        }
        digit = (unsigned long) (text.text[index] - '0');
        if (value > (limit - digit) / 10) {
            return 0;
        }
        value = value * 10 + digit;
    }
    if (whole == 0 || (point && given == 0)) {
        return 0;
    }
    for (; given < places; given++) {
        if (value > limit / 10) {
            return 0;
        }
        value *= 10;
    }

    if (negative && value > 0) {
        *out = -(croft_int) (value - 1) - 1;
        return 1;
    }
    *out = (croft_int) value;
    return 1;
}

void croft_decimal_to_string(croft_int value, croft_int places, char *out, croft_u16 *length, croft_u16 capacity)
{
    unsigned long scale = 1;
    unsigned long remaining;
    croft_int place;
    for (place = 0; place < places; place++) {
        scale *= 10;
    }
    remaining = croft_int_magnitude(value);
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
