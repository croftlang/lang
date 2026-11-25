
/* Decimals: whole numbers of hundredths for Decimal<2>, rounded half to even. */

static unsigned long magnitude(dn_int value)
{
    return value < 0 ? 0UL - (unsigned long) value : (unsigned long) value;
}

static dn_int with_sign(unsigned long value, dn_bool negative)
{
    return negative ? -(dn_int) value : (dn_int) value;
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

dn_int dn_divide_even(dn_int numerator, dn_int denominator)
{
    DN_CHECK(denominator != 0, "division by zero");
    if (denominator == 0) {
        return 0;
    }
    return with_sign(divide_even(magnitude(numerator), magnitude(denominator)),
                     (numerator < 0) != (denominator < 0));
}

/* a * b / scale, splitting a into whole and part so the product stays small. */
dn_int dn_decimal_multiply(dn_int a, dn_int b, dn_int scale)
{
    unsigned long left = magnitude(a);
    unsigned long right = magnitude(b);
    unsigned long whole = (left / (unsigned long) scale) * right;
    unsigned long part = divide_even((left % (unsigned long) scale) * right, (unsigned long) scale);
    return with_sign(whole + part, (a < 0) != (b < 0));
}

/* a * scale / b, as long division: the whole part, then the remainder scaled. */
dn_int dn_decimal_divide(dn_int a, dn_int b, dn_int scale)
{
    unsigned long left;
    unsigned long right;
    DN_CHECK(b != 0, "division by zero");
    if (b == 0) {
        return 0;
    }
    left = magnitude(a);
    right = magnitude(b);
    return with_sign((left / right) * (unsigned long) scale
                         + divide_even((left % right) * (unsigned long) scale, right),
                     (a < 0) != (b < 0));
}

void dn_decimal_to_string(dn_int value, dn_int places, dn_string *out)
{
    unsigned long scale = 1;
    unsigned long remaining;
    dn_int place;
    for (place = 0; place < places; place++) {
        scale *= 10;
    }
    remaining = magnitude(value);
    dn_int_to_string((dn_int) (remaining / scale), out);
    if (value < 0 && out->length < DN_STRING_CAPACITY - 1) {
        for (place = out->length; place > 0; place--) {
            out->text[place] = out->text[place - 1];
        }
        out->text[0] = '-';
        out->length++;
    }
    if (out->length < DN_STRING_CAPACITY) {
        out->text[out->length] = '.';
        out->length++;
    }
    remaining %= scale;
    while (scale > 1 && out->length < DN_STRING_CAPACITY) {
        scale /= 10;
        out->text[out->length] = (char) ('0' + remaining / scale);
        out->length++;
        remaining %= scale;
    }
}
