use crate::PackageException;
use crate::ScanEvent::{
    PackageArrivedAtSortingFacility, PackageDelivered, PackageOutForDelivery, ScanException,
    ShippingLabelCreated,
};
use crate::domain::ScanEvent;
use chrono::{DateTime, Utc};

/// A raw scan event as reported by the scanning hardware/carrier, before any
/// business interpretation has been applied. `code` is meaningless outside
/// of `interpret` below -- this is the one place allowed to know what a
/// given code actually means.
///
/// Deliberately a plain, hand-written type rather than a generated protobuf
/// struct: `ingest` owns decoding the wire format (protobuf-over-Kafka) and
/// is responsible for translating into this shape before calling into
/// `domain`. That keeps `domain` free of codegen/schema-evolution concerns,
/// the same way it stays free of `tokio`/`ractor`. See DESIGN.md.
pub struct RawScan {
    pub code: u32,
    pub timestamp: DateTime<Utc>,
    // TODO: facility/location id? carrier-specific metadata? decide once
    // you know what the real scan codes need to carry.
}

/// The lookup table: what does a given raw scan code actually mean?
///
/// TODO: decide whether an unrecognized code should be silently ignored
/// (`None`), or should be a `DomainError` -- an unknown code might indicate
/// an ingest/schema bug worth surfacing rather than swallowing.
pub fn interpret(raw: RawScan) -> Option<ScanEvent> {
    match raw.code {
        1000 => Some(ShippingLabelCreated {
            timestamp: raw.timestamp,
        }),
        2000 => Some(PackageArrivedAtSortingFacility {
            timestamp: raw.timestamp,
        }),
        3000 => Some(PackageOutForDelivery {
            timestamp: raw.timestamp,
        }),
        4000 => Some(PackageDelivered {
            timestamp: raw.timestamp,
            message: "Delivered to Recipient".to_string(),
        }),
        4001 => Some(PackageDelivered {
            timestamp: raw.timestamp,
            message: "Delivered to Mailbox".to_string(),
        }),
        5000 => Some(ScanException {
            timestamp: raw.timestamp,
            kind: PackageException::Damaged,
        }),
        5001 => Some(ScanException {
            timestamp: raw.timestamp,
            kind: PackageException::Lost,
        }),
        _ => None,
    }
}
