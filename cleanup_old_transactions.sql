-- Script to clean up old transactions with incorrect amounts from the bug

-- First, let's see what we have
SELECT id, reference, amount_from, amount_to, rate_applied, status, created_at
FROM transactions
WHERE amount_to > 100000  -- These are likely the buggy ones
ORDER BY created_at DESC;

-- Delete old pending transactions with incorrect amounts
DELETE FROM transactions
WHERE status = 'pending'
  AND amount_to > 100000  -- Bug produced huge EUR amounts
  AND created_at < NOW() - INTERVAL '1 hour';

-- Also delete any expired transactions older than 1 hour
DELETE FROM transactions
WHERE status = 'pending'
  AND expires_at < NOW() - INTERVAL '1 hour';

-- Check the results
SELECT COUNT(*) as remaining_bad_transactions
FROM transactions
WHERE amount_to > 100000;

-- Show summary of remaining transactions
SELECT 
    status,
    COUNT(*) as count,
    MIN(created_at) as oldest,
    MAX(created_at) as newest
FROM transactions
GROUP BY status
ORDER BY status;