// The outer layer: booking, routing and tracking cargo, and registering what
// the docks report.
context Shipping

enum ShippingError {
    include Cargo.Error
    include StorageError
    include NoRoute
}

// Books a cargo from Hong Kong to Helsinki, due in 30 days. It starts
// unrouted, waiting in Hong Kong to be received.
service SetUpBooking(): Result<Unit, StorageError> {
    calls Cargos
} = cargos.save(Cargo {
    tracking_id: SAMPLE_CARGO
    route: RouteSpecification { origin: "CNHKG", destination: "FIHEL", arrival_deadline: 30 }
    itinerary: Itinerary { legs: [] }
    delivery: Delivery { transport_status: NotReceived, last_location: None, current_voyage: None, misdirected: false }
})

// Asks Routing for a path that meets the route specification, and plans
// the cargo's legs along it.
service RouteCargo(tracking_id: TrackingId): Result<Unit, ShippingError> {
    calls Cargos
} = {
    let cargo = cargos.find(tracking_id)?
    let route = cargo.route
    let path = find_path(route.origin.value, route.destination.value, route.arrival_deadline.value) or NoRoute
    let routed = issue AssignToRoute(chosen: ItineraryFrom(path)) on cargo?
    cargos.save(routed)?
}

// Registers every report from the dock terminals until they have no more.
service RegisterReports(): Result<Unit, ShippingError> {
    calls HandlingReports
} = {
    loop {
        let next = handling_reports.next()
        if next.is_none {
            break
        }
        match next {
            Some(report) -> RegisterHandlingEvent(report)?
            None -> Unit
        }
    }
}

// Registers what a dock reported, dated today. The cargo must exist.
service RegisterHandlingEvent(report: HandlingReport): Result<Unit, ShippingError> {
    calls Cargos, HandlingEvents, Clock
} = {
    let cargo = cargos.find(report.tracking_id)?
    let handling = HandlingEvent {
        id: report.number
        tracking_id: cargo.tracking_id
        activity: report.activity
        completed_on: clock.today()
    }
    let registered = issue Register on handling?
    handling_events.save(registered)?
}

// Prints where the cargo is, for a customer asking after it.
service Track(tracking_id: TrackingId): Result<Unit, ShippingError> {
    calls Cargos, Console
} = {
    let cargo = cargos.find(tracking_id)?
    console.print(transport_text(cargo.delivery))
    console.print(routing_text(routing_status(cargo.itinerary, cargo.route)))
    if cargo.delivery.misdirected {
        console.print("Misdirected")
    }
}

fn transport_text(delivery: Delivery): String = {
    let place = delivery.last_location.map(location -> location.value).unwrap_or("?")
    match delivery.transport_status {
        NotReceived -> "Not received"
        InPort -> "In port at " + place
        OnboardCarrier -> "On voyage " + delivery.current_voyage.map(voyage -> voyage.value).unwrap_or("?")
        Claimed -> "Claimed at " + place
    }
}

fn routing_text(status: RoutingStatus): String = match status {
    NotRouted -> "Not routed"
    Routed -> "Routed"
    Misrouted -> "Misrouted"
}
