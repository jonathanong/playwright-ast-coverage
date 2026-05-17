-- Derived from sample/backend/data-stores/psql/views/2025-01-01-view-users.sql
-- Exercises: CREATE OR REPLACE VIEW

-- Lightweight embedded view for nesting in posts/topics JSON.
CREATE OR REPLACE VIEW view_embedded_users AS (
  SELECT
    'user' AS __entity_type,
    users.id,
    users.username,
    users.use_display_name_from,
    users.individual_id,
    users.profile_image_id
  FROM users
  WHERE users.deleted_at IS NULL
);

-- Thin alias over view_embedded_users for direct user queries.
CREATE OR REPLACE VIEW view_users_public AS (
  SELECT * FROM view_embedded_users
);
