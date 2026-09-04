//! Money-path fakes: payments, receipt templates, object storage, renderer.
use async_trait::async_trait;
use pg_application::error::AppError;
use pg_application::ports::storage::{ObjectStore, ReceiptData, ReceiptTemplate};
use pg_application::ports::tenancy::TenantContext;
use pg_application::ports::{
    AuditActor, PaymentRepository, ReceiptFile, ReceiptMeta, ReceiptTemplateRepository,
    RecordedPayment, VoidedPayment,
};
use pg_domain::{OrgId, PaymentId, ReceiptId, ResidentId, UserId};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

pub struct FakePaymentRepo {
    /// resident_id -> total paise recorded via `record`.
    pub paid: Mutex<HashMap<Uuid, i64>>,
    pub ready: Mutex<Vec<(Uuid, String)>>,
    /// resident_id -> the live payments in order, so a reversal can pop the
    /// latest one the way the real repository does.
    pub history: Mutex<HashMap<Uuid, Vec<(Uuid, i64, String)>>>,
}

impl FakePaymentRepo {
    pub fn new() -> Self {
        Self {
            paid: Mutex::new(HashMap::new()),
            ready: Mutex::new(Vec::new()),
            history: Mutex::new(HashMap::new()),
        }
    }
    pub fn total_for(&self, resident: Uuid) -> i64 {
        *self
            .paid
            .lock()
            .expect("poisoned")
            .get(&resident)
            .unwrap_or(&0)
    }
}

impl Default for FakePaymentRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PaymentRepository for FakePaymentRepo {
    async fn record(
        &self,
        _ctx: &TenantContext,
        pid: PaymentId,
        _rid: ReceiptId,
        resident: ResidentId,
        amount: i64,
        receipt_number: &str,
        _by: Option<UserId>,
        _method: &str,
        _reference: Option<&str>,
        _idempotency_key: Option<&str>,
        _actor: &AuditActor,
    ) -> Result<RecordedPayment, AppError> {
        *self
            .paid
            .lock()
            .expect("poisoned")
            .entry(resident.as_uuid())
            .or_insert(0) += amount;
        self.history
            .lock()
            .expect("poisoned")
            .entry(resident.as_uuid())
            .or_default()
            .push((pid.as_uuid(), amount, receipt_number.to_string()));
        Ok(RecordedPayment {
            payment_id: pid.as_uuid(),
            receipt_id: _rid.as_uuid(),
            receipt_number: receipt_number.to_string(),
            replayed: false,
        })
    }

    async fn void_last_payment(
        &self,
        _ctx: &TenantContext,
        resident: ResidentId,
        _actor: &AuditActor,
    ) -> Result<Option<VoidedPayment>, AppError> {
        let mut history = self.history.lock().expect("poisoned");
        let Some((payment_id, amount_paise, invoice_number)) = history
            .get_mut(&resident.as_uuid())
            .and_then(|entries| entries.pop())
        else {
            return Ok(None);
        };
        *self
            .paid
            .lock()
            .expect("poisoned")
            .entry(resident.as_uuid())
            .or_insert(0) -= amount_paise;
        Ok(Some(VoidedPayment {
            payment_id,
            amount_paise,
            invoice_number: Some(invoice_number),
        }))
    }
    async fn set_receipt_ready(
        &self,
        _ctx: &TenantContext,
        receipt_id: ReceiptId,
        storage_key: &str,
    ) -> Result<(), AppError> {
        self.ready
            .lock()
            .expect("poisoned")
            .push((receipt_id.as_uuid(), storage_key.to_string()));
        Ok(())
    }
    async fn paid_total(
        &self,
        _ctx: &TenantContext,
        resident: ResidentId,
    ) -> Result<i64, AppError> {
        Ok(*self
            .paid
            .lock()
            .expect("poisoned")
            .get(&resident.as_uuid())
            .unwrap_or(&0))
    }
    async fn list_receipts(
        &self,
        _ctx: &TenantContext,
        _r: ResidentId,
    ) -> Result<Vec<ReceiptMeta>, AppError> {
        unimplemented!("FakePaymentRepo::list_receipts")
    }
    async fn get_receipt(
        &self,
        _ctx: &TenantContext,
        _r: ReceiptId,
    ) -> Result<Option<ReceiptFile>, AppError> {
        unimplemented!("FakePaymentRepo::get_receipt")
    }
}

pub struct FakeReceiptTemplateRepo;

#[async_trait]
impl ReceiptTemplateRepository for FakeReceiptTemplateRepo {
    async fn resolve(&self, _org_id: OrgId) -> Result<ReceiptTemplate, AppError> {
        Ok(ReceiptTemplate::default_template())
    }
    async fn upsert(&self, _org_id: OrgId, _t: &ReceiptTemplate) -> Result<(), AppError> {
        unimplemented!("FakeReceiptTemplateRepo::upsert")
    }
}

/// Records the `ReceiptData` each render call received.
pub struct SpyRenderer {
    pub seen: Mutex<Vec<ReceiptData>>,
}

impl SpyRenderer {
    pub fn new() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
        }
    }
}

impl Default for SpyRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl pg_application::ports::ReceiptRenderer for SpyRenderer {
    fn render(&self, _template: &ReceiptTemplate, data: &ReceiptData) -> Vec<u8> {
        self.seen.lock().expect("poisoned").push(ReceiptData {
            receipt_number: data.receipt_number.clone(),
            invoice_number: data.invoice_number.clone(),
            resident_name: data.resident_name.clone(),
            branch_name: data.branch_name.clone(),
            amount_paise: data.amount_paise,
            issued_epoch: data.issued_epoch,
            tenant_phone: data.tenant_phone.clone(),
            room_number: data.room_number.clone(),
            plan_kind: data.plan_kind.clone(),
            line_item: data.line_item.clone(),
            payment_method: data.payment_method.clone(),
            payment_reference: None,
            floor_label: None,
            recorded_by: data.recorded_by.clone(),
        });
        b"%PDF-mock".to_vec()
    }
}

pub struct FakeObjectStore;

#[async_trait]
impl ObjectStore for FakeObjectStore {
    async fn put(&self, _key: &str, _bytes: Vec<u8>, _content_type: &str) -> Result<(), AppError> {
        Ok(())
    }
    async fn get(&self, _key: &str) -> Result<Vec<u8>, AppError> {
        unimplemented!("FakeObjectStore::get")
    }
    async fn signed_url(&self, _key: &str, _ttl: i64) -> Result<Option<String>, AppError> {
        unimplemented!("FakeObjectStore::signed_url")
    }
}
