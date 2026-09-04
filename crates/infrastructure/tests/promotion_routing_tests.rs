use pg_application::ports::tenancy::{Placement, TenantProvisioner, TenantResolver};
use pg_domain::OrgId;
use pg_infrastructure::{persistence::PgTenantProvisioner, tenancy::DbTenantResolver};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

#[tokio::test]
async fn promotion_moves_only_the_target_org_into_its_silo_and_updates_routing() {
    let Some(database_url) = std::env::var("MIGRATION_DATABASE_URL").ok() else {
        // Unit-test runs without an integration database remain deterministic.
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect integration database");
    let org = OrgId::new(Uuid::new_v4());
    let branch = Uuid::new_v4();
    let room = Uuid::new_v4();

    seed_pooled_org(&pool, org, branch, room).await;
    let result = verify_promotion(&pool, org, room).await;
    cleanup(&pool, org).await;
    result.expect("pool-to-silo routing must preserve only the target org data");
}

async fn seed_pooled_org(pool: &PgPool, org: OrgId, branch: Uuid, room: Uuid) {
    sqlx::query("INSERT INTO public.organizations (id, name) VALUES ($1, 'Promotion Routing QA')")
        .bind(org.as_uuid())
        .execute(pool)
        .await
        .expect("insert org");
    sqlx::query(
        "INSERT INTO public.tenant_placement (org_id, mode, schema_name) VALUES ($1, 'pooled', 'pool')",
    )
    .bind(org.as_uuid())
    .execute(pool)
    .await
    .expect("insert pooled placement");
    sqlx::query("INSERT INTO pool.branches (id, org_id, name) VALUES ($1, $2, 'QA Branch')")
        .bind(branch)
        .bind(org.as_uuid())
        .execute(pool)
        .await
        .expect("insert pooled branch");
    sqlx::query(
        "INSERT INTO pool.rooms (id, org_id, branch_id, number, cooling_type) VALUES ($1, $2, $3, '901', 'ac')",
    )
    .bind(room)
    .bind(org.as_uuid())
    .bind(branch)
    .execute(pool)
    .await
    .expect("insert pooled room");
}

async fn verify_promotion(pool: &PgPool, org: OrgId, room: Uuid) -> Result<(), String> {
    let provisioner = PgTenantProvisioner::new(pool.clone());
    let schema = provisioner
        .promote_to_silo(org)
        .await
        .map_err(|e| e.to_string())?;
    let resolver = DbTenantResolver::new(pool.clone());
    let context = resolver.resolve(org).await.map_err(|e| e.to_string())?;
    if context.placement
        != (Placement::Siloed {
            schema: schema.clone(),
        })
    {
        return Err("resolver did not return the promoted silo schema".into());
    }
    let copied: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM \"{schema}\".rooms WHERE id = $1"
    ))
    .bind(room)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    let remaining_in_pool: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pool.rooms WHERE id = $1")
            .bind(room)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
    if copied == 1 && remaining_in_pool == 0 {
        Ok(())
    } else {
        Err(format!(
            "expected silo=1 and pool=0, got silo={copied}, pool={remaining_in_pool}"
        ))
    }
}

async fn cleanup(pool: &PgPool, org: OrgId) {
    let schema = format!("org_{}", org.as_uuid().simple());
    let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM pool.rooms WHERE org_id = $1")
        .bind(org.as_uuid())
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM pool.branches WHERE org_id = $1")
        .bind(org.as_uuid())
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM public.tenant_placement WHERE org_id = $1")
        .bind(org.as_uuid())
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM public.organizations WHERE id = $1")
        .bind(org.as_uuid())
        .execute(pool)
        .await;
}
