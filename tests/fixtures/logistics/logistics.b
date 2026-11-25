// A shipping company's cargo system, after the example in Eric Evans'
// Domain-Driven Design. It books a cargo, routes it, registers what the
// docks report about it, then prints where it is.
domain Logistics {
    // Shipping finds routes through Routing; Platform fills Shipping's ports.
    contexts {
        Routing -> Shipping
        Shipping -> Platform
    }

    use { SetUpBooking, RouteCargo, RegisterReports, Track, SAMPLE_CARGO } from Shipping
} = {
    SetUpBooking()?
    RouteCargo(SAMPLE_CARGO)?
    RegisterReports()?
    Track(SAMPLE_CARGO)?
}
