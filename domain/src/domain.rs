use chrono::{DateTime, Days, Utc};

// Inbound event applied to package
enum ScanEvent {
    ShippingLabelCreated { at: DateTime<Utc> },
    ArrivedAtSortingFacility { at: DateTime<Utc> },
    OutForDeliver { at: DateTime<Utc> },
    Delivered { at: DateTime<Utc> },
    DeliveryException(String),
}

// Persisted event
enum PackageEvent {
    ShippingLabelCreated { at: DateTime<Utc> },
    ArrivedAtSortingFacility { at: DateTime<Utc> },
    OnRoute { at: DateTime<Utc> },
    Delivered { at: DateTime<Utc> },
    PackageException { message: String, at: DateTime<Utc> },
}

enum Status {
    PackageCreated,
    ArrivedAtSortingFacility,
    OnRoute,
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
        // TODO: This is arbitrary, need to build better EDD processing logic based on scans and current state
        let now = Utc::now();
        let now_plus_1_day = now.checked_add_days(Days::new(1));
        let now_plus_2_day = now.checked_add_days(Days::new(2));
        let now_plus_3_day = now.checked_add_days(Days::new(3));

        let (current_status, expected_delivery_date, timestamp) = match event {
            PackageEvent::ShippingLabelCreated { at } => (Status::PackageCreated, now_plus_3_day, at),
            PackageEvent::ArrivedAtSortingFacility { at } => {
                (Status::ArrivedAtSortingFacility, now_plus_2_day, at)
            }
            PackageEvent::OnRoute { at } => (Status::OnRoute, now_plus_1_day, at),
            PackageEvent::Delivered { at } => (Status::Delivered, None, at),
            PackageEvent::PackageException { message, at } => {
                (Status::PackageException(message.to_string()), None, at)
            }
        };

        PackageState {
            current_status,
            expected_delivery_date,
            last_scan: timestamp.to_utc(),
        }
    }

    fn process(&self, scan_event: ScanEvent) -> Result<PackageEvent, DomainError> {
        match scan_event {
            ScanEvent::ShippingLabelCreated { at } => Ok(PackageEvent::ArrivedAtSortingFacility { at }),
            ScanEvent::ArrivedAtSortingFacility { at } => {
                Ok(PackageEvent::ArrivedAtSortingFacility { at })
            }
            ScanEvent::OutForDeliver { at } => Ok(PackageEvent::ArrivedAtSortingFacility { at }),
            ScanEvent::Delivered { at } => Ok(PackageEvent::ArrivedAtSortingFacility { at }),
            ScanEvent::DeliveryException(message) => Err(DomainError::GeneralError { message }),
        }
    }
}
