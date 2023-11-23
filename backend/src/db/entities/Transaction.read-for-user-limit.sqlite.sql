SELECT uid, transaction_type, user_uid, description, amount, time
FROM Transactions
WHERE user_uid = ?
ORDER BY time DESC
LIMIT ?
OFFSET ?
