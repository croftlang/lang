//! C type definitions: structs for Croft types, sizes from constraints, and their order.

/// The bounds of strings, lists and bytes worked out from code.
pub mod bounds;
/// Working out constant expressions at compile time.
pub mod constant;
/// Type definitions in dependency order.
pub mod define;
/// Enum definitions.
pub mod enums;
/// The shared error type.
pub mod error;
/// The C type of each field.
pub mod fields;
/// Lists, maps, options and results.
pub mod generic;
/// How many bytes each type takes.
pub mod memory;
/// Sizes taken from constraints.
pub mod sizes;
/// How many bytes a whole number needs.
pub mod width;
