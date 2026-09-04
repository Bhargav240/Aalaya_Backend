//! Shared in-memory fakes for the application ports.
//!
//! One implementation per port, reused across every test suite in this crate.
//! Extend a fake by ADDING methods/behavior — never branch behavior inside an
//! existing method, and never define fakes inside individual test files.
//!
//! Persistence is `std::sync::Mutex`-guarded interiors so a single `&dyn Port`
//! reference stays `Send + Sync` under a multi-threaded Tokio runtime. Any port
//! method a suite does not exercise is left `unimplemented!()` so an accidental
//! dependency fails loudly during development instead of silently passing.
//!
//! Every suite compiles this module separately and picks what it needs; unused
//! warnings would fire for the rest, so we silence them module-wide.
#![allow(dead_code)]
#![allow(unused_imports)]
mod clock;
mod money;
mod repos;
mod tokens;

pub use clock::FixedClock;
pub use money::{FakeObjectStore, FakePaymentRepo, FakeReceiptTemplateRepo, SpyRenderer};
pub use repos::SeededResident;
pub use repos::{
    FakeAwayRepo, FakeBedRepo, FakeBranchRepo, FakeChangeRequestRepo, FakeRefreshTokenRepo,
    FakeResidentRepo, FakeRoomRepo, FakeSettlementRepo, FakeUserRepo, OrgRepoOk, StubResolver,
};

#[allow(unused_imports)]
pub use repos::TestUser;
#[allow(unused_imports)] // used by otp/auth suites, not every suite
pub use tokens::FakeTokenService;
