/* Std.Time through C89's time and localtime, for the SystemClock adapter.
   Written out only for a target that binds Clock to SystemClock. A machine
   with no clock gives 1 January 1970 at midnight, and a year past what a
   Date holds is kept to its first or last. */
#include <time.h>
#include "program.h"

/* The machine's local time, or 0 when it can't tell the time. */
static struct tm *local_time(void)
{
    time_t seconds = time(0);
    if (seconds == (time_t) -1) {
        return 0;
    }
    return localtime(&seconds);
}

void croft_time_today(Std_Time_Date *out)
{
    struct tm *local = local_time();
    long year;
    out->year = 1970;
    out->month = 1;
    out->day = 1;
    if (local == 0) {
        return;
    }
    year = (long) local->tm_year + 1900L;
    if (year < 1L) {
        year = 1L;
    }
    if (year > 9999L) {
        year = 9999L;
    }
    out->year = (int) year;
    out->month = local->tm_mon + 1;
    out->day = local->tm_mday;
}

void croft_time_now(Std_Time_Time *out)
{
    struct tm *local = local_time();
    out->hour = 0;
    out->minute = 0;
    out->second = 0;
    if (local == 0) {
        return;
    }
    out->hour = local->tm_hour;
    out->minute = local->tm_min;
    out->second = local->tm_sec > 59 ? 59 : local->tm_sec;
}
