INSERT INTO Transactions (transaction_type, user_uid, description, amount, time)
VALUES (?, ?, ?, ?, ?);

SELECT last_insert_rowid()
