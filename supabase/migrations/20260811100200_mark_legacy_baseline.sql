-- The Rust migration runner uses these markers to decide whether it must
-- bootstrap a database. Supabase has already applied the equivalent baseline
-- in the two preceding migrations, so record it without re-running legacy DDL.
INSERT INTO public.schema_migrations_tenant (schema_name, version)
VALUES
  ('public', '0001_init'),
  ('pool', '0001_init')
ON CONFLICT DO NOTHING;
