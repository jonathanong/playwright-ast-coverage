CREATE VIEW active_users AS
SELECT * FROM users WHERE active = true;

-- This is forbidden:
DROP VIEW IF EXISTS old_users;
