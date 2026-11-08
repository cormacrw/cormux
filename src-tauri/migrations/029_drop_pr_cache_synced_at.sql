-- Nothing read a cached pull request's sync time; when the list last synced is a setting.
ALTER TABLE pr_cache DROP COLUMN synced_at;
