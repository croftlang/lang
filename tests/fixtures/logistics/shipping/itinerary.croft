// Rules about itineraries that belong to no single cargo.
context Shipping

// Whether the itinerary takes the cargo from its origin to its destination
// in time.
fn satisfies(itinerary: Itinerary, route: RouteSpecification): Bool = match itinerary.legs.first {
    Some(first) -> match itinerary.legs.last {
        Some(last) -> first.load_at == route.origin && last.unload_at == route.destination && last.unload_day.value <= route.arrival_deadline.value
        None -> false
    }
    None -> false
}

fn routing_status(itinerary: Itinerary, route: RouteSpecification): RoutingStatus = {
    if itinerary.legs.is_empty {
        NotRouted
    }
    if satisfies(itinerary, route) {
        Routed
    }
    Misrouted
}

// Whether the itinerary expects the cargo to be handled this way. Until the
// cargo is routed, anything is expected.
fn expects(itinerary: Itinerary, activity: HandlingActivity): Bool = {
    if itinerary.legs.is_empty {
        true
    }
    match activity.kind {
        Receive -> itinerary.legs.first.map(leg -> leg.load_at == activity.location).unwrap_or(false)
        Load -> itinerary.legs.any(leg -> leg.load_at == activity.location && activity.voyage == Some(leg.voyage))
        Unload -> itinerary.legs.any(leg -> leg.unload_at == activity.location && activity.voyage == Some(leg.voyage))
        Claim -> itinerary.legs.last.map(leg -> leg.unload_at == activity.location).unwrap_or(false)
    }
}

fn status_after(kind: HandlingType): TransportStatus = match kind {
    Receive -> InPort
    Load -> OnboardCarrier
    Unload -> InPort
    Claim -> Claimed
}

// The voyage the cargo is on after being handled: only loading puts it on one.
fn voyage_after(activity: HandlingActivity): Option<VoyageNumber> = match activity.kind {
    Load -> activity.voyage
    _ -> None
}
