SELECT uid, role, name, email, family_uid
FROM Users
WHERE family_uid = ?
