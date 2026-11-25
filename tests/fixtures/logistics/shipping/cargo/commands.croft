// The commands that change a cargo.
context Shipping
aggregate Cargo

// Plans the cargo's legs. An itinerary that doesn't satisfy the route
// specification is still kept, and the cargo shows as misrouted.
command AssignToRoute(chosen: Itinerary) {
    delivery.transport_status != Claimed or AlreadyClaimed
} = {
    set itinerary = chosen
    emit Routed { itinerary: chosen }
}

// Sends the cargo somewhere else. Its itinerary may no longer get it there.
command ChangeDestination(destination: UnLocode) {
    delivery.transport_status != Claimed or AlreadyClaimed
} = {
    set route = route with { destination: destination }
    emit DestinationChanged { destination }
}

// Works out the delivery again after the cargo is handled.
command RecordHandling(activity: HandlingActivity) {
    delivery.transport_status != Claimed or AlreadyClaimed
} = {
    let misdirected = !expects(itinerary, activity)
    set delivery = Delivery {
        transport_status: status_after(activity.kind)
        last_location: Some(activity.location)
        current_voyage: voyage_after(activity)
        misdirected: misdirected
    }
    emit Handled { activity }
    if misdirected {
        emit Misdirected { location: activity.location }
    }
    if activity.kind == Unload && activity.location == route.destination {
        emit Arrived { at: activity.location }
    }
}
