// Adapters written in b, for running the program in tests.
context Platform

adapter FixedClock for Clock = {
    fn today() = 10
}

adapter ScriptedReports for HandlingReports = {
    // The sample cargo's journey from Hong Kong to Helsinki, changing at
    // Long Beach, until the customer claims it.
    reports: List<HandlingReport> where Length(0, 8) = [
        HandlingReport { number: 1, tracking_id: 1, activity: HandlingActivity { kind: Receive, location: "CNHKG", voyage: None } },
        HandlingReport { number: 2, tracking_id: 1, activity: HandlingActivity { kind: Load, location: "CNHKG", voyage: Some(VoyageNumber("V100")) } },
        HandlingReport { number: 3, tracking_id: 1, activity: HandlingActivity { kind: Unload, location: "USLBG", voyage: Some(VoyageNumber("V100")) } },
        HandlingReport { number: 4, tracking_id: 1, activity: HandlingActivity { kind: Load, location: "USLBG", voyage: Some(VoyageNumber("V200")) } },
        HandlingReport { number: 5, tracking_id: 1, activity: HandlingActivity { kind: Unload, location: "FIHEL", voyage: Some(VoyageNumber("V200")) } },
        HandlingReport { number: 6, tracking_id: 1, activity: HandlingActivity { kind: Claim, location: "FIHEL", voyage: None } }
    ]

    // Gives the next scripted report, then forgets it.
    fn next() = match reports.first {
        Some(report) -> {
            set reports = reports.drop(1)
            Some(report)
        }
        None -> None
    }
}
