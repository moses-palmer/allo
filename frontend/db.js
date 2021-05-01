/**
 * The name of the database to use.
 */
const NAME = "allo";

/**
 * Our current schema version
 */
const VERSION = 1;

/**
 * The object store names.
 */
const T = {
    /**
     * Application state.
     *
     * This object store contains objects keyed on `uid`, and one unique ID
     * where the key is "me".
     */
    STATE: "state",
};


export class Database {
    /**
     * Constructs a database instance.
     */
    constructor(db) {
        this._db = db;
    }

    /**
     * Loads the current application state from the database.
     *
     * @param uid
     *     The unique ID of the state. If this is not specified, the ID is read
     *     from the database using the key "me".
     */
    async load(uid) {
        const trans = this._db.transaction(T.STATE);
        if (uid !== undefined) {
            return await get(trans, T.STATE, uid)
        } else {
            const me = await get(trans, T.STATE, "me");
            if (me !== undefined) {
                return await get(trans, T.STATE, me);
            } else {
                return undefined;
            }
        }
    }

    /**
     * Stores the current state into the database.
     *
     * This function does nothing unless `state.me.uid` is set.
     */
    async store(state) {
        const trans = this._db.transaction(T.STATE, "readwrite");
        const uid = state.me?.uid;
        if (uid !== undefined) {
            await put(trans, T.STATE, uid, state);
            await put(trans, T.STATE, "me", uid);
        }
    }

    /**
     * Clears the entire database.
     */
    async clear() {
        const trans = this._db.transaction(T.STATE, "readwrite");
        await promise(trans.objectStore(T.STATE).clear());
    }
}


/**
 * Opens a connection to the database.
 *
 * This method ensures that the database is upgraded.
 *
 * @returns a `Database` instance
 * @throws an error event wrapped as `{type: eventName, event: eventData}`,
 *     where `eventName` is "error" or "blocked"
 */
export default async () => {
    const req = indexedDB.open(Database.NAME, Database.VERSION);
    return new Database((await new Promise((resolve, reject) => {
        req.addEventListener("error", reject);
        req.addEventListener("success", resolve);
        req.addEventListener(
            "upgradeneeded",
            async (e) => {
                await upgrade(req.result, e.oldVersion);
                resolve(e);
            });
    })).target.result);
};


/**
 * Reads a value from an object store.
 *
 * @param trans
 *     An open transaction.
 * @param store
 *     The name of an object store.
 * @param key
 *     The key of the item to retrieve.
 * @returns an item
 * @throws an error event wrapped as `{type: eventName, event: eventData}`
 */
const get = async (trans, store, key) => (await promise(
    trans.objectStore(store).get(key))).target.result;


/**
 * Writes a value to an object store.
 *
 * @param trans
 *     An open transaction.
 * @param store
 *     The name of an object store.
 * @param key
 *     The key of the item to write.
 * @param value
 *     The value to put.
 * @throws an error event wrapped as `{type: eventName, event: eventData}`
 */
const put = async (trans, store, key, value) => await promise(
    trans.objectStore(store).put(value, key));


/**
 * Waits for an `IDBRequest` to fulfil.
 *
 * @param req
 *     The request to await.
 * @returns a success event
 * @throws an error event
 */
const promise = async (req) => await new Promise(
    (resolve, reject) => {
        req.addEventListener("success", resolve);
        req.addEventListener("error", reject)
    });


/**
 * Upgrades the database to to the current schema.
 *
 * @param db
 *     The database instance.
 * @param fromVersion
 *     The version from which to upgrade.
 */
const upgrade = async (db, fromVersion) => {
    const v1 = async () => await promise(db.createObjectStore(T.STATE));

    switch (fromVersion) {
    case 0:
        await v1();
        // Fall-through
    case 1:
        // Current
        return;
    default:
        console.error(`unknown database version: ${fromVersion}`);
    }
};
