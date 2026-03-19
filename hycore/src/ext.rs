use downcast_rs::{Downcast, impl_downcast};

/// Empty trait to mark objects that are extension objects, that is portion of the API that is fully extendable
pub trait ExtObject: Downcast {}
impl_downcast!(ExtObject);
