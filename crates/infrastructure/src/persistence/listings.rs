use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::discovery::{Listing, ListingInput, ListingRepository, ListingStatus};
use pg_application::AppError;
use pg_domain::{BranchId, OrgId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgListingRepository {
    pool: PgPool,
}

impl PgListingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type Row = (
    Uuid,
    Uuid,
    String,
    Option<String>,
    Option<String>,
    Option<f64>,
    Option<f64>,
    Option<String>,
    Option<String>,
    String,
    Option<i32>,
    Option<i64>,
    Option<f64>,
);

fn to_listing(r: Row) -> Listing {
    Listing {
        branch_id: r.0,
        org_id: r.1,
        title: r.2,
        area: r.3,
        city: r.4,
        lat: r.5,
        lng: r.6,
        description: r.7,
        amenities: r.8,
        status: ListingStatus::parse(&r.9).unwrap_or(ListingStatus::Draft),
        beds_available: r.10,
        starting_price_paise: r.11,
        distance_km: r.12,
    }
}

// Vacancy is JOINED live from the projector's snapshot — never hand-typed.
const COLS: &str = "l.branch_id, l.org_id, l.title, l.area, l.city, l.lat, l.lng, \
     l.description, l.amenities, l.status, v.beds_available, v.starting_price_paise";

#[async_trait]
impl ListingRepository for PgListingRepository {
    async fn upsert_draft(
        &self,
        org_id: OrgId,
        branch_id: BranchId,
        input: &ListingInput,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.public_listings \
             (branch_id, org_id, title, area, city, lat, lng, description, amenities, status, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'draft', now()) \
             ON CONFLICT (branch_id) DO UPDATE SET org_id=EXCLUDED.org_id, title=EXCLUDED.title, \
             area=EXCLUDED.area, city=EXCLUDED.city, lat=EXCLUDED.lat, lng=EXCLUDED.lng, \
             description=EXCLUDED.description, amenities=EXCLUDED.amenities, updated_at=now()",
        )
        .bind(branch_id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(&input.title)
        .bind(&input.area)
        .bind(&input.city)
        .bind(input.lat)
        .bind(input.lng)
        .bind(&input.description)
        .bind(&input.amenities)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn set_status(
        &self,
        branch_id: BranchId,
        status: ListingStatus,
        note: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE public.public_listings SET status=$2, rejection_note=$3, updated_at=now(), \
             published_at = CASE WHEN $2='live' THEN now() ELSE published_at END \
             WHERE branch_id=$1",
        )
        .bind(branch_id.as_uuid())
        .bind(status.as_str())
        .bind(note)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_by_org(&self, org_id: OrgId) -> Result<Vec<Listing>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS}, NULL::double precision FROM public.public_listings l \
             LEFT JOIN public.listing_vacancy v ON v.branch_id = l.branch_id \
             WHERE l.org_id=$1 ORDER BY l.updated_at DESC LIMIT 500"
        ))
        .bind(org_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(to_listing).collect())
    }

    async fn list_by_status(&self, status: ListingStatus) -> Result<Vec<Listing>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS}, NULL::double precision FROM public.public_listings l \
             LEFT JOIN public.listing_vacancy v ON v.branch_id = l.branch_id \
             WHERE l.status=$1 ORDER BY l.updated_at ASC"
        ))
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(to_listing).collect())
    }

    async fn list_published(&self, limit: i64) -> Result<Vec<Listing>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS}, NULL::double precision FROM public.public_listings l \
             LEFT JOIN public.listing_vacancy v ON v.branch_id = l.branch_id \
             WHERE l.status='live' ORDER BY COALESCE(v.beds_available,0) DESC, l.updated_at DESC LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(to_listing).collect())
    }

    async fn search(
        &self,
        q: Option<String>,
        city: Option<String>,
        limit: i64,
    ) -> Result<Vec<Listing>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS}, NULL::double precision FROM public.public_listings l \
             LEFT JOIN public.listing_vacancy v ON v.branch_id = l.branch_id \
             WHERE l.status='live' \
             AND ($1 IS NULL OR (l.title ILIKE '%'||$1||'%' OR l.area ILIKE '%'||$1||'%' \
                  OR l.city ILIKE '%'||$1||'%' OR COALESCE(l.description,'') ILIKE '%'||$1||'%')) \
             AND ($2 IS NULL OR l.city ILIKE $2) ORDER BY l.updated_at DESC LIMIT $3"
        ))
        .bind(q)
        .bind(city)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(to_listing).collect())
    }

    async fn nearby(&self, lat: f64, lng: f64, limit: i64) -> Result<Vec<Listing>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS}, \
             (6371 * acos(LEAST(1, cos(radians($1)) * cos(radians(l.lat)) * cos(radians(l.lng) - radians($2)) \
              + sin(radians($1)) * sin(radians(l.lat))))) AS distance_km \
             FROM public.public_listings l \
             LEFT JOIN public.listing_vacancy v ON v.branch_id = l.branch_id \
             WHERE l.status='live' AND l.lat IS NOT NULL AND l.lng IS NOT NULL \
             ORDER BY distance_km ASC LIMIT $3"
        ))
        .bind(lat)
        .bind(lng)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(to_listing).collect())
    }

    async fn get(&self, branch_id: Uuid) -> Result<Option<Listing>, AppError> {
        let row: Option<Row> = sqlx::query_as(&format!(
            "SELECT {COLS}, NULL::double precision FROM public.public_listings l \
             LEFT JOIN public.listing_vacancy v ON v.branch_id = l.branch_id \
             WHERE l.branch_id = $1"
        ))
        .bind(branch_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(to_listing))
    }
}
