INSERT INTO Transactions (transaction_type, user_uid, description,
        amount, time)
    SELECT ?, user_uid, '', amount, ?
    FROM Allowances
    WHERE schedule = ?
