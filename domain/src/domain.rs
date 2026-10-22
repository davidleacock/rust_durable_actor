#![allow(dead_code)] // TODO: remove once variants/fields are filled in

use crate::PackageEvent::{
    EDDUpdated, Exception, PackageDelivered, PhysicallyScanned, StatusUpdated,
};
use chrono::{DateTime, Days, Utc};
// ---------------------------------------------------------------------------
// Inbound domain event -- the output of `scan_code::interpret`, and the
// input to `PackageState::process`. Carries business meaning, unlike the
// `RawScan` it was built from.
//
// Lesson from the last pass: this doesn't need to be 1:1 with `PackageEvent`
// below -- a scan is valid input even when it results in zero persisted
// facts (e.g. a stale/out-of-order scan might decide to emit nothing).
// ---------------------------------------------------------------------------

// TODO - Will keep these events basic for now, but with the option of more minute detailed
// events to come

pub enum ScanEvent {
    ShippingLabelCreated {
        timestamp: DateTime<Utc>,
    },
    PackageArrivedAtSortingFacility {
        timestamp: DateTime<Utc>,
    },
    PackageOutForDelivery {
        timestamp: DateTime<Utc>,
    },
    PackageDelivered {
        timestamp: DateTime<Utc>,
        message: String,
    },
    ScanException {
        timestamp: DateTime<Utc>,
        kind: PackageException,
    },
}

// ---------------------------------------------------------------------------
// Persisted event -- what actually goes in the event log and gets published
// downstream. This is the source of truth; `PackageState` is just a fold
// over these.
// ---------------------------------------------------------------------------
pub enum PackageEvent {
    StatusUpdated {
        status: Status,
        timestamp: DateTime<Utc>,
    },
    PhysicallyScanned {
        timestamp: DateTime<Utc>,
    },
    EDDUpdated {
        estimated_delivery_date: DateTime<Utc>,
        timestamp: DateTime<Utc>,
    },
    Exception {
        package_exception: PackageException,
        timestamp: DateTime<Utc>,
    },
    ExceptionResolved {
        timestamp: DateTime<Utc>,
    },
    PackageDelivered {
        message: String,
    },
}

// ---------------------------------------------------------------------------
// Where a package currently is in its lifecycle. Last design used a linear
// `rank()` comparison on this to detect stale/out-of-order scans -- worth
// deciding again whether that's still the right tool, or whether it needs
// rethinking now that scans arrive as raw codes.
// ---------------------------------------------------------------------------
#[derive(Copy, Clone)]
pub enum Status {
    PackageCreated,
    PackageEnroute,
    PackageOutForDelivery,
    PackageDelivered,
}

// ---------------------------------------------------------------------------
// Deliberately NOT a `Status` variant. An open exception is orthogonal to
// how far along the package is -- a package can be "at the sorting
// facility" AND "flagged lost" at the same time. Forcing exception into
// `Status`'s linear order was the bug in the last design; keep it split out
// on `PackageState` instead (e.g. `Option<PackageException>`).
// ---------------------------------------------------------------------------
#[derive(Copy, Clone)]
pub enum PackageException {
    Damaged,
    Lost,
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    // TODO
}

pub struct PackageState {
    status: Status,
    timestamp: DateTime<Utc>,
    last_physical_scan: Option<DateTime<Utc>>,
    estimated_delivery_date: Option<DateTime<Utc>>,
    delivery_notes: Option<String>,
    package_exception: Option<PackageException>,
}

impl PackageState {
    /// Decide what, if anything, should be persisted in response to a scan.
    ///
    /// Takes `&self`, not `self`: this only needs to *read* current state to
    /// make a decision, and the caller still needs ownership afterward to
    /// fold the resulting events via `apply`. Returns `Vec` rather than a
    /// single event (or `Option`) because a real command handler can
    /// legitimately decide to persist zero, one, or several facts from one
    /// scan.
    pub fn process(&self, scan_event: ScanEvent) -> Result<Vec<PackageEvent>, DomainError> {
        match scan_event {
            ScanEvent::ShippingLabelCreated { timestamp } => Ok(vec![StatusUpdated {
                status: Status::PackageCreated,
                timestamp,
            }]),
            ScanEvent::PackageArrivedAtSortingFacility { timestamp } => Ok(vec![
                PhysicallyScanned { timestamp },
                StatusUpdated {
                    status: Status::PackageEnroute,
                    timestamp,
                },
                EDDUpdated {
                    // TODO this is an arbitrary bump of 5 days, could make it more specific later
                    estimated_delivery_date: timestamp
                        .checked_add_days(Days::new(5))
                        .unwrap()
                        .to_utc(),
                    timestamp,
                },
            ]),
            ScanEvent::PackageOutForDelivery { timestamp } => Ok(vec![
                PhysicallyScanned { timestamp },
                StatusUpdated {
                    status: Status::PackageOutForDelivery,
                    timestamp,
                },
                EDDUpdated {
                    // Package is out for delivery, so the EDD is today
                    estimated_delivery_date: timestamp,
                    timestamp,
                },
            ]),
            ScanEvent::PackageDelivered { timestamp, message } => Ok(vec![
                StatusUpdated {
                    status: Status::PackageDelivered,
                    timestamp,
                },
                PackageDelivered { message },
            ]),
            ScanEvent::ScanException { timestamp, kind } => Ok(vec![Exception {
                package_exception: kind,
                timestamp,
            }]),
        }
    }

    /// Fold one persisted event into state.
    ///
    /// Takes `self` by value: this is a pure replace, not a mutation. MUST
    /// be fully deterministic -- no `Utc::now()`, no I/O, no randomness.
    /// Every event gets replayed on every actor recovery, so anything
    /// computed here has to be a pure function of `(self, event)`, or
    /// replaying the same log twice will produce two different states.
    pub fn apply(self, event: &PackageEvent) -> PackageState {
        match event {
            PackageEvent::StatusUpdated { status, timestamp } => PackageState {
                status: *status,
                timestamp: *timestamp,
                last_physical_scan: self.last_physical_scan,
                estimated_delivery_date: self.estimated_delivery_date,
                delivery_notes: self.delivery_notes,
                package_exception: self.package_exception,
            },
            PackageEvent::PhysicallyScanned { timestamp } => PackageState {
                status: self.status,
                timestamp: *timestamp,
                last_physical_scan: Some(*timestamp),
                estimated_delivery_date: self.estimated_delivery_date,
                delivery_notes: self.delivery_notes,
                package_exception: self.package_exception,
            },
            PackageEvent::EDDUpdated {
                estimated_delivery_date,
                timestamp,
            } => PackageState {
                status: self.status,
                timestamp: *timestamp,
                last_physical_scan: self.last_physical_scan,
                estimated_delivery_date: Some(*estimated_delivery_date),
                delivery_notes: self.delivery_notes,
                package_exception: self.package_exception,
            },
            PackageEvent::Exception {
                package_exception,
                timestamp,
            } => PackageState {
                status: self.status,
                timestamp: *timestamp,
                last_physical_scan: self.last_physical_scan,
                estimated_delivery_date: self.estimated_delivery_date,
                delivery_notes: self.delivery_notes,
                package_exception: Some(*package_exception),
            },
            PackageEvent::ExceptionResolved { timestamp } => PackageState {
                status: self.status,
                timestamp: *timestamp,
                last_physical_scan: self.last_physical_scan,
                estimated_delivery_date: self.estimated_delivery_date,
                delivery_notes: self.delivery_notes,
                package_exception: None,
            },
            PackageEvent::PackageDelivered { message } => PackageState {
                status: self.status,
                timestamp: self.timestamp,
                last_physical_scan: self.last_physical_scan,
                estimated_delivery_date: self.estimated_delivery_date,
                delivery_notes: Some(message.clone()),
                package_exception: self.package_exception,
            },
        }
    }
}

// TODO: Stage 1's definition of done wants unit tests covering the happy
// path, an out-of-order/duplicate scan, and an invalid transition. Add a
// `#[cfg(test)] mod tests` once `process`/`apply` have real bodies.
