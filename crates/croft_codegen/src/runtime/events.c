/* The event queue. A command queues its events as not yet kept; saving the
   aggregate that raised them keeps them. When a service ends, its kept events
   go to the policies that react to them, and the rest are dropped, since they
   describe changes that were never saved. */
croft_event croft_events[CROFT_EVENT_CAPACITY];
croft_u16 croft_event_count = 0;

croft_u16 croft_events_mark(void)
{
    return croft_event_count;
}

void croft_events_rollback(croft_u16 mark)
{
    croft_event_count = mark;
}

void croft_events_push(const croft_event *event)
{
    CROFT_CHECK(croft_event_count < CROFT_EVENT_CAPACITY, "too many events at once");
    if (croft_event_count < CROFT_EVENT_CAPACITY) {
        croft_events[croft_event_count] = *event;
        croft_events[croft_event_count].kept = 0;
        croft_event_count++;
    }
}

/* A queued event, for a factory to fill in what only its finished aggregate knows. */
croft_event *croft_events_at(croft_u16 index)
{
    return &croft_events[index];
}

void croft_events_keep(croft_u16 aggregate, croft_int key)
{
    croft_u16 index;
    for (index = 0; index < croft_event_count; index++) {
        if (croft_events[index].aggregate == aggregate && croft_events[index].key == key) {
            croft_events[index].kept = 1;
        }
    }
}

static void croft_events_handle(const croft_event *event)
{
/* HANDLERS */    (void) event;
}

void croft_events_dispatch_from(croft_u16 mark)
{
    croft_event event;
    croft_u16 index;

    while (croft_event_count > mark) {
        event = croft_events[mark];
        for (index = mark + 1; index < croft_event_count; index++) {
            croft_events[index - 1] = croft_events[index];
        }
        croft_event_count--;
        if (event.kept) {
            croft_events_handle(&event);
        }
    }
}
