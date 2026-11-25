/* The event queue. A command queues its events as not yet kept; saving the
   aggregate that raised them keeps them. When a service ends, its kept events
   go to the policies that react to them, and the rest are dropped, since they
   describe changes that were never saved. */
b_event b_events[B_EVENT_CAPACITY];
b_u16 b_event_count = 0;

b_u16 b_events_mark(void)
{
    return b_event_count;
}

void b_events_rollback(b_u16 mark)
{
    b_event_count = mark;
}

void b_events_push(const b_event *event)
{
    B_CHECK(b_event_count < B_EVENT_CAPACITY, "too many events at once");
    if (b_event_count < B_EVENT_CAPACITY) {
        b_events[b_event_count] = *event;
        b_events[b_event_count].kept = 0;
        b_event_count++;
    }
}

void b_events_keep(b_u16 aggregate, b_int key)
{
    b_u16 index;
    for (index = 0; index < b_event_count; index++) {
        if (b_events[index].aggregate == aggregate && b_events[index].key == key) {
            b_events[index].kept = 1;
        }
    }
}

static void b_events_handle(const b_event *event)
{
/* HANDLERS */    (void) event;
}

void b_events_dispatch_from(b_u16 mark)
{
    b_event event;
    b_u16 index;

    while (b_event_count > mark) {
        event = b_events[mark];
        for (index = mark + 1; index < b_event_count; index++) {
            b_events[index - 1] = b_events[index];
        }
        b_event_count--;
        if (event.kept) {
            b_events_handle(&event);
        }
    }
}
