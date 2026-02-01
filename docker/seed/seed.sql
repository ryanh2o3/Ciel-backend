INSERT INTO users (id, handle, email, display_name, bio, password_hash)
VALUES
    ('00000000-0000-0000-0000-000000000001', 'demo', 'demo@example.com', 'Demo User', 'Hello from PicShare.', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$RdescudvJC1OeqEcglpmXw'),
    ('00000000-0000-0000-0000-000000000002', 'alice', 'alice@example.com', 'Alice', 'Coffee, photos, and travel.', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$RdescudvJC1OeqEcglpmXw'),
    ('00000000-0000-0000-0000-000000000003', 'bob', 'bob@example.com', 'Bob', 'Street photography enthusiast.', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$RdescudvJC1OeqEcglpmXw'),
    ('00000000-0000-0000-0000-000000000004', 'cora', 'cora@example.com', 'Cora', 'Food, friends, and sunsets.', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$RdescudvJC1OeqEcglpmXw')
ON CONFLICT DO NOTHING;

INSERT INTO follows (follower_id, followee_id)
VALUES
    ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000002'),
    ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000003'),
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000003'),
    ('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000004')
ON CONFLICT DO NOTHING;

INSERT INTO media (id, owner_id, original_key, thumb_key, medium_key, width, height, bytes, created_at)
VALUES
    ('00000000-0000-0000-0000-000000010001', '00000000-0000-0000-0000-000000000002', 'seed/alice/coffee.jpg', 'seed/alice/coffee_thumb.jpg', 'seed/alice/coffee_medium.jpg', 1600, 1200, 245000, now() - interval '3 days'),
    ('00000000-0000-0000-0000-000000010002', '00000000-0000-0000-0000-000000000003', 'seed/bob/city.jpg', 'seed/bob/city_thumb.jpg', 'seed/bob/city_medium.jpg', 2000, 1333, 310000, now() - interval '2 days'),
    ('00000000-0000-0000-0000-000000010003', '00000000-0000-0000-0000-000000000004', 'seed/cora/plate.jpg', 'seed/cora/plate_thumb.jpg', 'seed/cora/plate_medium.jpg', 1400, 1400, 275000, now() - interval '1 days')
ON CONFLICT DO NOTHING;

INSERT INTO posts (id, owner_id, media_id, caption, visibility, created_at)
VALUES
    ('00000000-0000-0000-0000-000000020001', '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000010001', 'Morning coffee.', 'public', now() - interval '3 days'),
    ('00000000-0000-0000-0000-000000020002', '00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000010002', 'City lights.', 'public', now() - interval '2 days'),
    ('00000000-0000-0000-0000-000000020003', '00000000-0000-0000-0000-000000000004', '00000000-0000-0000-0000-000000010003', 'Dinner vibes.', 'public', now() - interval '1 days')
ON CONFLICT DO NOTHING;

INSERT INTO likes (id, user_id, post_id, created_at)
VALUES
    ('00000000-0000-0000-0000-000000030001', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000020001', now() - interval '2 days'),
    ('00000000-0000-0000-0000-000000030002', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000020002', now() - interval '1 days'),
    ('00000000-0000-0000-0000-000000030003', '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000020002', now() - interval '1 days')
ON CONFLICT DO NOTHING;

INSERT INTO comments (id, user_id, post_id, body, created_at)
VALUES
    ('00000000-0000-0000-0000-000000040001', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000020001', 'Looks great!', now() - interval '2 days'),
    ('00000000-0000-0000-0000-000000040002', '00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000020003', 'Nice colors.', now() - interval '1 days')
ON CONFLICT DO NOTHING;
