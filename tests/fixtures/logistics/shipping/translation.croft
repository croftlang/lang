// The anti-corruption layer between Routing's model and ours. Routing
// speaks of nodes and edges; Shipping speaks of locations, voyages and legs.
context Shipping

use { Edge, Path, find_path } from Routing

translation ItineraryFrom(path: Path): Itinerary = Itinerary {
    legs: path.edges.map(edge -> LegFrom(edge))
}

translation LegFrom(edge: Edge): Leg = Leg {
    voyage: VoyageNumber(edge.voyage)
    load_at: UnLocode(edge.from_node)
    unload_at: UnLocode(edge.to_node)
    load_day: Day(edge.departs)
    unload_day: Day(edge.arrives)
}
