/* The event queue. A command queues its events as not yet kept; saving the
   aggregate that raised them keeps them. When a service ends, its kept events
   go to the policies that react to them, and the rest are dropped, since they
   describe changes that were never saved. */
dn_event dn_events[DN_EVENT_CAPACITY];
dn_u16 dn_event_count = 0;

dn_u16 dn_events_mark(void)
{
    return dn_event_count;
}

void dn_events_rollback(dn_u16 mark)
{
    dn_event_count = mark;
}

void dn_events_push(const dn_event *event)
{
    DN_CHECK(dn_event_count < DN_EVENT_CAPACITY, "too many events at once");
    if (dn_event_count < DN_EVENT_CAPACITY) {
        dn_events[dn_event_count] = *event;
        dn_events[dn_event_count].kept = 0;
        dn_event_count++;
    }
}

void dn_events_keep(dn_u16 aggregate, dn_int key)
{
    dn_u16 index;
    for (index = 0; index < dn_event_count; index++) {
        if (dn_events[index].aggregate == aggregate && dn_events[index].key == key) {
            dn_events[index].kept = 1;
        }
    }
}

static void dn_events_handle(const dn_event *event)
{
/* HANDLERS */    (void) event;
}

void dn_events_dispatch_from(dn_u16 mark)
{
    dn_event event;
    dn_u16 index;

    while (dn_event_count > mark) {
        event = dn_events[mark];
        for (index = mark + 1; index < dn_event_count; index++) {
            dn_events[index - 1] = dn_events[index];
        }
        dn_event_count--;
        if (event.kept) {
            dn_events_handle(&event);
        }
    }
}
