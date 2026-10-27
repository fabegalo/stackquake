//! Data source connectors. Each source is an isolated module; to add another,
//! create a module here and call it from `sync.rs`.

pub mod crashlytics;
pub mod firebase;
pub mod play;
