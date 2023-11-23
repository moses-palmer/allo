SELECT user_uid, hash
FROM Passwords
LEFT JOIN Users
    ON Passwords.user_uid = Users.uid
WHERE Users.email = ?
