//! Strongly-typed identifiers. A `BranchId` can never be passed where an
//! `OrgId` is expected -- the compiler enforces it.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new(inner: Uuid) -> Self {
                Self(inner)
            }
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

id_type!(OrgId);
id_type!(BranchId);
id_type!(RoomId);
id_type!(FloorId);
id_type!(BedId);
id_type!(UserId);
id_type!(ResidentId);
id_type!(ComplaintId);
id_type!(NoticeId);
id_type!(PaymentId);
id_type!(ReceiptId);
