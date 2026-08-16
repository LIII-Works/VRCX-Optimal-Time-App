CREATE TABLE usr550e8400e29b41d4a716446655440000_feed_avatar (created_at TEXT NOT NULL, user_id TEXT, display_name TEXT, type TEXT);
CREATE TABLE usr550e8400e29b41d4a716446655440000_feed_gps (created_at TEXT NOT NULL, user_id TEXT, display_name TEXT, type TEXT);
CREATE TABLE usr550e8400e29b41d4a716446655440000_feed_online_offline (created_at TEXT NOT NULL, user_id TEXT, display_name TEXT, type TEXT);
CREATE TABLE usr550e8400e29b41d4a716446655440000_feed_status (created_at TEXT NOT NULL, user_id TEXT, display_name TEXT, type TEXT);
CREATE TABLE usr550e8400e29b41d4a716446655440000_friend_log_history (created_at TEXT NOT NULL, user_id TEXT, display_name TEXT, type TEXT);
INSERT INTO usr550e8400e29b41d4a716446655440000_feed_avatar (created_at) VALUES ('2024-01-01T00:00:00Z'), ('2024-01-01T00:10:00Z'), ('2024-01-01T00:20:00Z'), ('2024-01-01T00:30:00Z');
INSERT INTO usr550e8400e29b41d4a716446655440000_feed_online_offline (created_at, user_id, display_name, type) VALUES ('2024-01-01T00:05:00Z', 'usr_friend', 'Friend', 'Online'), ('2024-01-01T00:25:00Z', 'usr_friend', 'Friend', 'Offline');
