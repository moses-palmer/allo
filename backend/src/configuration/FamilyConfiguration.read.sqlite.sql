SELECT family_uid, Currencies.name as name, Currencies.format as format
FROM Configurations
LEFT JOIN Currencies
    ON Configurations.currency = Currencies.name
WHERE Configurations.family_uid = ?
