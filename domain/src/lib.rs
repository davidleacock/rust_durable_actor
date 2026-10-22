mod domain;
mod scan_code;

pub use domain::{DomainError, PackageEvent, PackageException, PackageState, ScanEvent, Status};
pub use scan_code::{interpret, RawScan};
