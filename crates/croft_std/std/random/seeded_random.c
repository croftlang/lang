/* Std.Random through a xorshift32 generator, for the SeededRandom adapter.
   Written out only for a target that binds Random to SeededRandom. Its
   arithmetic is on unsigned longs kept to 32 bits, which C89 gives every
   machine, 8-bit ones included. */
#include <time.h>
#include "program.h"

/* Keeps a number to its lowest 32 bits, where an unsigned long has more. */
#define CROFT_RANDOM_BITS 0xFFFFFFFFUL

/* The generator's state, which is never 0 once it is seeded. */
static unsigned long croft_random_state = 0;

/* The next 32 random bits. The first call seeds the state from the clock,
   or from a fixed number where the clock gives 0. */
static unsigned long croft_random_next(void)
{
    unsigned long state;
    if (croft_random_state == 0) {
        croft_random_state = ((unsigned long) time(0)) & CROFT_RANDOM_BITS;
        if (croft_random_state == 0) {
            croft_random_state = 2463534242UL;
        }
    }
    state = croft_random_state;
    state ^= (state << 13) & CROFT_RANDOM_BITS;
    state ^= state >> 17;
    state ^= (state << 5) & CROFT_RANDOM_BITS;
    croft_random_state = state;
    return state;
}

/* A number from low to high, both included, with every one as likely: a
   draw below the threshold is thrown away, so the remainder has no bias.
   The checks on the port have already made sure low is not above high. */
void croft_random_between(croft_int low, croft_int high, croft_int *out)
{
    unsigned long span;
    unsigned long threshold;
    unsigned long draw;
    unsigned long offset;
    span = (((unsigned long) high - (unsigned long) low) & CROFT_RANDOM_BITS) + 1UL;
    span &= CROFT_RANDOM_BITS;
    draw = croft_random_next();
    if (span == 0) {
        /* Every 32-bit number is allowed, so each draw is used whole. */
        offset = draw;
    } else {
        threshold = ((CROFT_RANDOM_BITS - span) + 1UL) % span;
        while (draw < threshold) {
            draw = croft_random_next();
        }
        offset = draw % span;
    }
    /* low + offset is at most high, but offset alone can be past
       CROFT_INT_MAX when low is negative, so it is added in two steps. */
    if (offset > (unsigned long) CROFT_INT_MAX) {
        *out = (croft_int) (low + CROFT_INT_MAX);
        *out = (croft_int) (*out + (croft_int) (offset - (unsigned long) CROFT_INT_MAX));
        return;
    }
    *out = (croft_int) (low + (croft_int) offset);
}
