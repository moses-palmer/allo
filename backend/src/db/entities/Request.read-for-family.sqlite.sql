SELECT Requests.uid, user_uid, Requests.name, description, amount, url, time
FROM Requests
LEFT JOIN Users
    ON Requests.user_uid = Users.uid
WHERE Users.family_uid = ?
