// What shipping needs from the systems around it.
context Shipping

// Reports from the dock terminals, one at a time.
port HandlingReports {
    fn next(): Option<HandlingReport>
}

port Clock {
    fn today(): Day
}

repository Cargos for Cargo
