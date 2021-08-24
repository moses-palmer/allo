/**
 * Invitations for new users.
 */
CREATE TABLE Invitations (
    /**
     * The unique ID.
     *
     * This is represented by the type db::values::UID.
     */
    uid TEXT PRIMARY KEY UNIQUE,

    /**
     * The future family role.
     *
     * This is represented by the type db::values::Role.
     */
    role TEXT NOT NULL,

    /**
     * The display name.
     */
    name TEXT NOT NULL,

    /**
     * The email address.
     *
     * This is represented by the type db::values::EmailAddress.
     */
    email TEXT NOT NULL,

    /**
     * The allowance amount, if this user is a child.
     */
    allowance_amount INTEGER,

    /**
     * The schedule for transactions, if this user is a child.
     */
    allowance_schedule TEXT,

    /**
     * The timestamp of this invitation.
     */
    time DATETIME NOT NULL,

    /**
     * The unique ID of the family.
     */
    family_uid TEXT NOT NULL,

    FOREIGN KEY (family_uid)
        REFERENCES Families (uid)
        ON DELETE CASCADE
);

