SELECT uid, role, name, email, allowance_amount, allowance_schedule, time,
    family_uid
FROM Invitations
WHERE family_uid = ?
