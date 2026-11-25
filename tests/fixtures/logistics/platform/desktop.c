/* Native functions for the desktop adapters. The prototypes are in
   program.h. */
#include <stdio.h>
#include <string.h>
#include <time.h>
#include "program.h"

/* The season began on 1 January 2026, at midnight UTC. */
#define SEASON_START 1767225600L
#define SECONDS_PER_DAY 86400L
#define LAST_DAY 3650L

void clock_today(Shipping_Day *out)
{
    long days = ((long) time(NULL) - SEASON_START) / SECONDS_PER_DAY;
    if (days < 0) {
        days = 0;
    }
    if (days > LAST_DAY) {
        days = LAST_DAY;
    }
    out->value = (b_u16) days;
}

static void copy_text(char *into, b_u16 *length, const char *text, unsigned int capacity)
{
    unsigned int size = (unsigned int) strlen(text);
    if (size > capacity) {
        size = capacity;
    }
    memcpy(into, text, size);
    *length = (b_u16) size;
}

static int handling_type(const char *word, Shipping_HandlingType *out)
{
    if (strcmp(word, "receive") == 0) {
        *out = Shipping_HandlingType_Receive;
    } else if (strcmp(word, "load") == 0) {
        *out = Shipping_HandlingType_Load;
    } else if (strcmp(word, "unload") == 0) {
        *out = Shipping_HandlingType_Unload;
    } else if (strcmp(word, "claim") == 0) {
        *out = Shipping_HandlingType_Claim;
    } else {
        return 0;
    }
    return 1;
}

/* Reads one report a line from standard input, such as
   `3 1 unload USLBG V100`: the report's number, the cargo's tracking id,
   what was done, where, and for loading and unloading, the voyage. Lines
   that can't be read are skipped. */
void terminal_next_report(b_option_Shipping_HandlingReport *out)
{
    char line[128];
    char kind[16];
    char location[16];
    char voyage[16];
    unsigned int number;
    unsigned int tracking_id;
    int read;
    Shipping_HandlingActivity *activity = &out->value.activity;

    b_clear(out, sizeof *out);
    while (fgets(line, sizeof line, stdin) != NULL) {
        voyage[0] = '\0';
        read = sscanf(line, "%u %u %15s %15s %15s", &number, &tracking_id, kind, location, voyage);
        if (read < 4 || number < 1 || tracking_id < 1 || strlen(location) != 5
            || !handling_type(kind, &activity->kind)) {
            continue;
        }
        out->value.number = (Shipping_HandlingEventId) number;
        out->value.tracking_id = (Shipping_TrackingId) tracking_id;
        copy_text(activity->location.value.text, &activity->location.value.length, location, 5);
        if (read == 5) {
            activity->voyage.some = 1;
            copy_text(activity->voyage.value.value.text, &activity->voyage.value.value.length, voyage, 4);
        }
        out->some = 1;
        return;
    }
}
