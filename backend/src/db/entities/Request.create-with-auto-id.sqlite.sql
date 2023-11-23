INSERT INTO Requests (user_uid, name, description, amount, url, time)
VALUES (?, ?, ?, ?, ?, ?);

SELECT last_insert_rowid()
