//! Thread features may save a helical B-rep thread: `helical`, a custom `pitch`, `depth`,
//! `angle`, `clearance`, `start_offset` and `end_offset`. Older designs have none
//! of them and keep their cosmetic or faceted threads unchanged. A release without them would
//! drop the fields and rebuild the thread as a faceted ISO one (or fail on a custom pitch), so it
//! must ask for an update instead.
