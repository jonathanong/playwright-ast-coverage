-- Derived from sample/backend/data-stores/psql/migrations/0001-00-00-users-and-auth.sql
-- Exercises: UUID PK, FK columns, CHECK constraints, triggers, timestamp columns, ENUMs
-- pg-schema: no-table-comments

-- ============================================================================
-- ENUMs
-- ============================================================================

CREATE TYPE user_display_name_source AS ENUM (
  'username',
  'facebook',
  'x'
);

CREATE TYPE user_privacy_audiences AS ENUM ('everyone', 'users', 'followers', 'mutual_followers', 'nobody');

CREATE TYPE broadcast_types AS ENUM ('everyone', 'users', 'followers', 'mutual_followers');
CREATE TYPE privacy_types AS ENUM ('public', 'private');

-- ============================================================================
-- users
-- ============================================================================

CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT uuidv7(),

  -- public
  use_display_name_from user_display_name_source DEFAULT 'username',
  username TEXT DEFAULT NULL,
  CHECK (username IS NULL OR char_length(username) <= 255),
  CHECK (username IS NULL OR username = TRIM(username)),

  -- user profile
  markdown TEXT NOT NULL DEFAULT '',

  -- user settings
  third_party_marketing BOOLEAN DEFAULT NULL,

  -- privacy settings
  cards_visibility user_privacy_audiences NOT NULL DEFAULT 'everyone',
  follows_visibility user_privacy_audiences NOT NULL DEFAULT 'everyone',
  default_post_broadcast broadcast_types NOT NULL DEFAULT 'everyone',
  default_post_privacy privacy_types NOT NULL DEFAULT 'public',

  -- referral attribution
  referrer_id UUID REFERENCES users(id) ON DELETE SET NULL,

  -- GDPR right to restrict processing
  processing_restricted_at TIMESTAMPTZ,

  -- suspension
  suspended_at TIMESTAMPTZ,
  suspended_reason TEXT,
  suspended_by_id UUID REFERENCES users(id) ON DELETE SET NULL,

  -- onboarding
  onboarding_completed_at TIMESTAMPTZ,

  created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(id)) VIRTUAL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  deleted_at TIMESTAMPTZ,
  deleted_by_id UUID REFERENCES users ON DELETE SET NULL
);

CREATE OR REPLACE TRIGGER trigger_users_updated_at
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION fn_update_updated_at();

CREATE UNIQUE INDEX idx_users__username
ON users (LOWER(username))
WHERE username IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_users__referrer_id ON users (referrer_id) WHERE referrer_id IS NOT NULL;

-- ============================================================================
-- email addresses
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_email_addresses (
  user_id UUID REFERENCES users ON DELETE CASCADE,
  email_address TEXT NOT NULL,
  CHECK (char_length(email_address) <= 255),
  CHECK (email_address = LOWER(email_address)),
  CHECK (email_address = TRIM(email_address)),
  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  is_primary BOOLEAN DEFAULT TRUE,
  PRIMARY KEY (user_id, email_address)
);

CREATE OR REPLACE TRIGGER trigger_user_email_addresses_updated_at
BEFORE UPDATE ON user_email_addresses
FOR EACH ROW
EXECUTE FUNCTION fn_update_updated_at();
