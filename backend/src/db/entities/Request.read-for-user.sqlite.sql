SELECT Requests.uid, user_uid, Requests.name, description, amount, url, time
FROM Requests
WHERE user_uid = ?
