/* The b runtime. Every generated program includes this header.
   Written in C89 so it builds with cc65, z88dk and old Atari ST compilers. */
#ifndef B_H
#define B_H

typedef long b_int;
typedef unsigned char b_bool;
typedef unsigned char b_unit;
typedef unsigned char b_u8;
typedef signed char b_i8;
typedef unsigned short b_u16;
typedef short b_i16;
typedef long b_i32;

#ifndef B_STRING_CAPACITY
#define B_STRING_CAPACITY 32
#endif

typedef struct {
    b_u16 length;
    char text[B_STRING_CAPACITY];
} b_string;

#ifndef B_BYTES_CAPACITY
#define B_BYTES_CAPACITY 64
#endif

typedef struct {
    b_u16 length;
    unsigned char items[B_BYTES_CAPACITY];
} b_bytes;

#define B_MAX(a, b) ((a) > (b) ? (a) : (b))
#define B_MIN(a, b) ((a) < (b) ? (a) : (b))
#define B_ABS(a) ((a) < 0 ? -(a) : (a))
/* Through unsigned long, since C leaves shifting a negative number left undefined. */
#define B_SHIFT_LEFT(value, amount) ((b_int) ((unsigned long) (value) << (amount)))
/* C89 leaves shifting a negative number right up to the compiler, so the
   sign is kept by hand: ~ makes it positive, and ~ again after the shift. */
#define B_SHIFT_RIGHT(value, amount) \
    ((b_int) ((value) < 0 ? ~(~(value) >> (amount)) : (value) >> (amount)))

/* Stops the program when a contract is broken. */
void b_fail(const char *message);

#ifdef B_DEBUG
#define B_CHECK(condition, message) if (!(condition)) b_fail(message)
#else
/* Not worked out, but still a use of what it names, for the compiler. */
#define B_CHECK(condition, message) (void) sizeof(condition)
#endif

/* Sets every byte of a value to zero, so unused parts are never read unset. */
void b_clear(void *value, unsigned int size);

void b_string_set(b_string *out, const char *text);
b_bool b_string_equal(const b_string *a, const b_string *b);
void b_string_join(const b_string *a, const b_string *b, b_string *out);
b_int b_string_find(const b_string *text, const b_string *part, b_int from);
b_bool b_string_starts_with(const b_string *text, const b_string *part);
b_bool b_string_ends_with(const b_string *text, const b_string *part);
void b_string_to_upper(const b_string *text, b_string *out);
void b_string_to_lower(const b_string *text, b_string *out);
void b_string_trim(const b_string *text, b_string *out);
void b_string_slice(const b_string *text, b_int start, b_int end, b_string *out);
void b_int_to_string(b_int value, b_string *out);

b_int b_divide_even(b_int numerator, b_int denominator);
b_int b_decimal_multiply(b_int a, b_int b, b_int scale);
b_int b_decimal_divide(b_int a, b_int b, b_int scale);
void b_decimal_to_string(b_int value, b_int places, b_string *out);

b_bool b_bytes_equal(const b_bytes *a, const b_bytes *b);
b_bool b_bytes_get(const b_bytes *bytes, b_int index, b_int *out);
void b_bytes_add(const b_bytes *bytes, b_int byte, b_bytes *out);
void b_bytes_slice(const b_bytes *bytes, b_int start, b_int end, b_bytes *out);
void b_bytes_to_text(const b_bytes *bytes, b_string *out);
void b_string_to_bytes(const b_string *text, b_bytes *out);
b_bool b_bytes_put_int(b_bytes *out, b_int value);
b_bool b_bytes_take_int(const b_bytes *in, b_u16 *at, b_int *out);
b_bool b_bytes_put_length(b_bytes *out, b_u16 length);
b_bool b_bytes_take_length(const b_bytes *in, b_u16 *at, b_u16 *out);
b_bool b_bytes_put_string(b_bytes *out, const b_string *text);
b_bool b_bytes_take_string(const b_bytes *in, b_u16 *at, b_string *out);
b_bool b_bytes_put_bytes(b_bytes *out, const b_bytes *bytes);
b_bool b_bytes_take_bytes(const b_bytes *in, b_u16 *at, b_bytes *out);
void b_storage_key(const char *prefix, b_int identity, b_string *out);

void b_text_load(const char *text, b_u16 length, b_string *out);
b_bool b_text_store(const b_string *in, char *text, b_u16 *length, b_u16 capacity);
b_bool b_text_equal(const char *a, b_u16 a_length, const char *b, b_u16 b_length);
b_bool b_bytes_put_text(b_bytes *out, const char *text, b_u16 length);
b_bool b_bytes_take_text(const b_bytes *in, b_u16 *at, char *text, b_u16 *length,
                           b_u16 capacity);
b_bool b_string_to_int(const b_string *text, b_int *out);

void b_arguments_set(int count, char **values);
b_int b_argument_count(void);
void b_argument(b_int index, b_string *out);

/* The built-in Console port: prints a line. */
void b_console_print(const b_string *text, b_unit *out);

#endif
