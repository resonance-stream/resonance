-- Resonance: Queue Prefetch Index
-- Migration: 20250101000014_queue_prefetch_index
--
-- Adds a partial index for efficient prefetch queries.
-- The worker's prefetch job queries for unprefetched tracks frequently,
-- and this index avoids a sequential scan of the JSONB metadata.

-- Partial index for items that haven't been prefetched yet
-- Covers the common case where prefetch status is NULL or explicitly false
CREATE INDEX idx_queue_items_unprefetched ON queue_items(user_id, position)
WHERE metadata->>'prefetched' IS DISTINCT FROM 'true';

-- Comment for documentation
COMMENT ON INDEX idx_queue_items_unprefetched IS 'Partial index for efficient prefetch queries - covers unprefetched queue items';
