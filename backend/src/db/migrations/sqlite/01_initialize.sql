/**
 * The various currencies supported.
 */
CREATE TABLE Configurations (
    /**
     * The unique ID of the family using this configuration.
     */
    family_uid TEXT PRIMARY KEY UNIQUE,

    /**
     * The name of the currency used by this famiily.
     */
    currency TEXT NOT NULL,

    FOREIGN KEY (family_uid)
        REFERENCES Families (uid)
        ON DELETE CASCADE,
    FOREIGN KEY (currency)
        REFERENCES Currencies (name)
        ON DELETE RESTRICT
);

/**
 * The various currencies supported.
 */
CREATE TABLE Currencies (
    /**
     * The name of this currency.
     */
    name TEXT PRIMARY KEY UNIQUE,

    /**
     * A format string to use to stringify values in this currency.
     *
     * This is represented by the type db::values::CurrencyFormat.
     */
    format TEXT NOT NULL
);

/**
 * The families using this application.
 */
CREATE TABLE Families (
    /**
     * The unique ID.
     *
     * This is represented by the type db::values::UID.
     */
    uid TEXT PRIMARY KEY UNIQUE,

    /**
     * The display name.
     */
    name TEXT NOT NULL
);

/**
 * The users of this application.
 */
CREATE TABLE Users (
    /**
     * The unique ID.
     *
     * This is represented by the type db::values::UID.
     */
    uid TEXT PRIMARY KEY UNIQUE,

    /**
     * The user family role.
     *
     * This is represented by the type db::values::Role.
     */
    role TEXT NOT NULL,

    /**
     * The display name.
     */
    name TEXT NOT NULL,

    /**
     * The mail address.
     *
     * This is represented by the type db::values::ValidatedEmailAddress.
     */
    email TEXT UNIQUE,

    /**
     * The unique ID of the family.
     */
    family_uid TEXT NOT NULL,

    FOREIGN KEY (family_uid)
        REFERENCES Families (uid)
        ON DELETE CASCADE
);

/**
 * The passwords.
 */
CREATE TABLE Passwords (
    /**
     * The unique ID of the user.
     */
    user_uid TEXT PRIMARY KEY UNIQUE,

    /**
     * The password hash.
     */
    hash TEXT NOT NULL,

    FOREIGN KEY (user_uid)
        REFERENCES Users (uid)
        ON DELETE CASCADE
);

/**
 * The transactions that have happened.
 */
CREATE TABLE Transactions (
    /**
     * The unique ID.
     */
    uid INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,

    /**
     * The type of transaction.
     */
    transaction_type TEXT NOT NULL,

    /**
     * The user involved in the transaction.
     */
    user_uid TEXT NOT NULL,

    /**
     * A description of the transaction.
     */
    description TEXT NOT NULL,

    /**
     * The amount.
     */
    amount INTEGER NOT NULL,

    /**
     * The timestamp of this transaction.
     */
    time DATETIME NOT NULL,

    FOREIGN KEY (user_uid)
        REFERENCES Users (uid)
        ON DELETE CASCADE
);

/**
 * A request for withdrawal.
 */
CREATE TABLE Requests (
    /**
     * The unique ID.
     */
    uid INTEGER PRIMARY KEY AUTOINCREMENT UNIQUE,

    /**
     * The user making the request.
     */
    user_uid TEXT NOT NULL,

    /**
     * A short name.
     */
    name TEXT NOT NULL,

    /**
     * A longer description.
     */
    description TEXT NOT NULL,

    /**
     * The amount.
     */
    amount INTEGER NOT NULL,

    /**
     * An optional associated URL.
     */
    url TEXT,

    /**
     * The timestamp of this request.
     */
    time DATETIME NOT NULL,

    FOREIGN KEY (user_uid)
        REFERENCES Users (uid)
        ON DELETE CASCADE
);

CREATE TABLE Allowances (
    /**
     * The unique ID.
     *
     * This is represented by the type db::values::UID.
     */
    uid TEXT PRIMARY KEY UNIQUE,

    /**
     * The user that is receiving this allowance.
     */
    user_uid TEXT NOT NULL,

    /**
     * The amount.
     */
    amount INTEGER NOT NULL,

    /**
     * The schedule for transactions.
     */
    schedule TEXT NOT NULL,

    FOREIGN KEY (user_uid)
        REFERENCES Users (uid)
        ON DELETE CASCADE
);

/**
 * A log of execution times for scheduled tasks.
 */
CREATE TABLE ScheduledTasks (
    /**
     * The task name.
     */
    task TEXT NOT NULL,

    /**
     * The timestamp representation, used to determine when the task should be
     * rerun.
     */
    last_run TEXT NOT NULL,

    /**
     * The actual timestamp for the last run.
     */
    time DATETIME NOT NULL
);
