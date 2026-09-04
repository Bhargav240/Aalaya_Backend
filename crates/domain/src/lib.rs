//! Pure business types and rules. No framework, no I/O, no external services.
pub mod entities;
pub mod enums;
pub mod errors;
pub mod ids;

pub use entities::{Organization, User};
pub use enums::{
    ChangeRequestStatus, ComplaintState, InvitationStatus, PlanKind, Role, UserStatus,
};
pub use errors::DomainError;
pub use ids::{
    BedId, BranchId, ComplaintId, FloorId, NoticeId, OrgId, PaymentId, ReceiptId, ResidentId,
    RoomId, UserId,
};
