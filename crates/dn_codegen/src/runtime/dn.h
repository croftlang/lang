/* The dn runtime. Every generated program includes this header.
   Written in C89 so it builds with cc65, z88dk and old Atari ST compilers. */
#ifndef DN_H
#define DN_H

typedef long dn_int;
typedef unsigned char dn_bool;
typedef unsigned char dn_unit;
typedef unsigned char dn_u8;
typedef signed char dn_i8;
typedef unsigned short dn_u16;
typedef short dn_i16;
typedef long dn_i32;

#ifndef DN_STRING_CAPACITY
#define DN_STRING_CAPACITY 32
#endif

typedef struct {
    dn_u16 length;
    char text[DN_STRING_CAPACITY];
} dn_string;

#ifndef DN_BYTES_CAPACITY
#define DN_BYTES_CAPACITY 64
#endif

typedef struct {
    dn_u16 length;
    unsigned char items[DN_BYTES_CAPACITY];
} dn_bytes;

#define DN_MAX(a, b) ((a) > (b) ? (a) : (b))
#define DN_MIN(a, b) ((a) < (b) ? (a) : (b))
#define DN_ABS(a) ((a) < 0 ? -(a) : (a))
/* Through unsigned long, since C leaves shifting a negative number left undefined. */
#define DN_SHIFT_LEFT(value, amount) ((dn_int) ((unsigned long) (value) << (amount)))
/* C89 leaves shifting a negative number right up to the compiler, so the
   sign is kept by hand: ~ makes it positive, and ~ again after the shift. */
#define DN_SHIFT_RIGHT(value, amount) \
    ((dn_int) ((value) < 0 ? ~(~(value) >> (amount)) : (value) >> (amount)))

/* Stops the program when a contract is broken. */
void dn_fail(const char *message);

#ifdef DN_DEBUG
#define DN_CHECK(condition, message) if (!(condition)) dn_fail(message)
#else
/* Not worked out, but still a use of what it names, for the compiler. */
#define DN_CHECK(condition, message) (void) sizeof(condition)
#endif

/* Sets every byte of a value to zero, so unused parts are never read unset. */
void dn_clear(void *value, unsigned int size);

void dn_string_set(dn_string *out, const char *text);
dn_bool dn_string_equal(const dn_string *a, const dn_string *b);
void dn_string_join(const dn_string *a, const dn_string *b, dn_string *out);
dn_int dn_string_find(const dn_string *text, const dn_string *part, dn_int from);
dn_bool dn_string_starts_with(const dn_string *text, const dn_string *part);
dn_bool dn_string_ends_with(const dn_string *text, const dn_string *part);
void dn_string_to_upper(const dn_string *text, dn_string *out);
void dn_string_to_lower(const dn_string *text, dn_string *out);
void dn_string_trim(const dn_string *text, dn_string *out);
void dn_string_slice(const dn_string *text, dn_int start, dn_int end, dn_string *out);
void dn_int_to_string(dn_int value, dn_string *out);

dn_int dn_divide_even(dn_int numerator, dn_int denominator);
dn_int dn_decimal_multiply(dn_int a, dn_int b, dn_int scale);
dn_int dn_decimal_divide(dn_int a, dn_int b, dn_int scale);
void dn_decimal_to_string(dn_int value, dn_int places, dn_string *out);

dn_bool dn_bytes_equal(const dn_bytes *a, const dn_bytes *b);
dn_bool dn_bytes_get(const dn_bytes *bytes, dn_int index, dn_int *out);
void dn_bytes_add(const dn_bytes *bytes, dn_int byte, dn_bytes *out);
void dn_bytes_slice(const dn_bytes *bytes, dn_int start, dn_int end, dn_bytes *out);
void dn_bytes_to_text(const dn_bytes *bytes, dn_string *out);
void dn_string_to_bytes(const dn_string *text, dn_bytes *out);
dn_bool dn_bytes_put_int(dn_bytes *out, dn_int value);
dn_bool dn_bytes_take_int(const dn_bytes *in, dn_u16 *at, dn_int *out);
dn_bool dn_bytes_put_length(dn_bytes *out, dn_u16 length);
dn_bool dn_bytes_take_length(const dn_bytes *in, dn_u16 *at, dn_u16 *out);
dn_bool dn_bytes_put_string(dn_bytes *out, const dn_string *text);
dn_bool dn_bytes_take_string(const dn_bytes *in, dn_u16 *at, dn_string *out);
dn_bool dn_bytes_put_bytes(dn_bytes *out, const dn_bytes *bytes);
dn_bool dn_bytes_take_bytes(const dn_bytes *in, dn_u16 *at, dn_bytes *out);
void dn_storage_key(const char *prefix, dn_int identity, dn_string *out);
dn_bool dn_string_to_int(const dn_string *text, dn_int *out);

void dn_arguments_set(int count, char **values);
dn_int dn_argument_count(void);
void dn_argument(dn_int index, dn_string *out);

/* The built-in Console port: prints a line. */
void dn_console_print(const dn_string *text, dn_unit *out);

#endif
