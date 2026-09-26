use chrono::{DateTime, Days, Utc};

//  TODO - ScanEvents should be more obtuse to reflect the real world, they're in the domain of scanning tools
// Inbound event applied to package
enum ScanEvent {
    ShippingLabelCreated { at: DateTime<Utc> },
    ArrivedAtSortingFacility { at: DateTime<Utc> },
    OnRoute { at: DateTime<Utc> },
    OutForDeliver { at: DateTime<Utc> },
    Delivered { at: DateTime<Utc> },
    DeliveryException(String),
}

// Persisted event
enum PackageEvent {
    ShippingLabelCreated { at: DateTime<Utc> },
    ArrivedAtSortingFacility { at: DateTime<Utc> },
    OnRoute { at: DateTime<Utc> },
    OutForDeliver { at: DateTime<Utc> },
    Delivered { at: DateTime<Utc> },
    PackageException { message: String, at: DateTime<Utc> },
}

enum Status {
    PackageCreated,
    ArrivedAtSortingFacility,
    OnRoute,
    OutForDeliver,
    Delivered,
    PackageException(String),
}

#[derive(Debug, thiserror::Error)]
enum DomainError {
    #[error("general error: {message}")]
    GeneralError { message: String },
}

struct PackageState {
    current_status: Status,
    expected_delivery_date: Option<DateTime<Utc>>,
    last_scan: DateTime<Utc>,
}

impl PackageState {
    fn apply(self, event: &PackageEvent) -> PackageState {
        //TODO - consider self

        let (current_status, expected_delivery_date, timestamp) = match event {
            PackageEvent::ShippingLabelCreated { at } => {
                (Status::PackageCreated, Some(*at + Days::new(3)), at)
            }
            PackageEvent::ArrivedAtSortingFacility { at } => (
                Status::ArrivedAtSortingFacility,
                Some(*at + Days::new(2)),
                at,
            ),
            PackageEvent::OnRoute { at } => (Status::OnRoute, Some(*at + Days::new(1)), at),
            PackageEvent::Delivered { at } => (Status::Delivered, None, at),
            PackageEvent::PackageException { message, at } => {
                (Status::PackageException(message.to_string()), None, at)
            }
            PackageEvent::OutForDeliver { at } => {
                (Status::OutForDeliver, Some(*at + Days::new(0)), at)
            }
        };

        PackageState {
            current_status,
            expected_delivery_date,
            last_scan: timestamp.to_utc(),
        }
    }

    fn process(&self, scan_event: ScanEvent) -> Result<PackageEvent, DomainError> {
        // TODO - Consider the case of duplicate events, or out-of-order events

        match scan_event {
            ScanEvent::ShippingLabelCreated { at } => {
                Ok(PackageEvent::ArrivedAtSortingFacility { at })
            }
            ScanEvent::ArrivedAtSortingFacility { at } => {
                Ok(PackageEvent::ArrivedAtSortingFacility { at })
            }
            ScanEvent::OnRoute { at } => Ok(PackageEvent::OnRoute { at }),
            ScanEvent::OutForDeliver { at } => Ok(PackageEvent::OutForDeliver { at }),
            ScanEvent::Delivered { at } => Ok(PackageEvent::ArrivedAtSortingFacility { at }),
            ScanEvent::DeliveryException(message) => Err(DomainError::GeneralError { message }),
        }
    }
}
