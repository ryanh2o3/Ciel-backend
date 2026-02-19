-- Allow system-generated invite codes (no owning user)
ALTER TABLE invite_codes ALTER COLUMN created_by DROP NOT NULL;
