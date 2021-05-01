import { translate as _ } from "./translation.js";

/**
 * The base URL for API requests.
 */
export const BASE_URL = "api/";

/**
 * The default error handler.
 *
 * This method is called when the server responds with a status code of 500 or
 * greater.
 */
let DEFAULT_ERROR_HANDLER = async (e) => alert(e);


const module = {
    /**
     * Sets the default error handler.
     *
     * @param handler
     *     The new default error handler.
     */
    defaultErrorHandler: (handler) => {
        DEFAULT_ERROR_HANDLER = handler;
    },

    /**
     * The class added to the body when a connection error occurs.
     */
    ERROR_CLASS: "error",

    transaction: {
        /**
         * Creates a new transaction.
         *
         * @param state
         *     The application state.
         * @param user
         *     The unique ID of the user involved in the transaction.
         * @param type
         *     The transaction type.
         * @param amount
         *     The amount.
         * @param description
         *     A description of the transaction.
         */
        create: (state, user, type, amount, description) => module.post(
            "transaction/{}".format(user), {
                transaction_type: type,
                amount,
                description}),

        /**
         * Lists transactions for a user.
         *
         * @param state
         *     The application state.
         * @param user
         *     The unique ID of the user involved in the transaction.
         * @param offset
         *     The offset from the last transaction from which to read.
         * @param limit
         *     The maximum number of transactions to return. The actual number
         *     may be smaller.
         */
        list: (state, user, offset, limit) => module.get(
            "transaction/{}?offset={}&limit={}".format(user, offset, limit)),
    },

    request: {
        /**
         * Declines a request.
         *
         * @param state
         *     The application state.
         * @param user
         *     The unique ID of the user making the request.
         * @param uid
         *     The unique ID of the request.
         * @return a future
         */
        decline: (state, user, uid) => module.remove(
            "request/{}/{}".format(user, uid)),

        /**
         * Gets a specific request.
         *
         * @param state
         *     The application state.
         * @param user
         *     The unique ID of the user who made the request.
         * @param request
         *     The request ID.
         */
        get: (state, user, request) => module.get(
            "request/{}/{}".format(user, request)),

        /**
         * Grants a request.
         *
         * @param state
         *     The application state.
         * @param user
         *     The unique ID of the user making the request.
         * @param uid
         *     The unique ID of the request.
         * @param cost
         *     An optional cost to override the value in the request.
         * @return a future
         */
        grant: (state, user, uid, cost) => module.post(
            "request/{}/{}".format(user, uid), {
                cost,
            }),

        /**
         * Creates a request.
         *
         * @param state
         *     The application state.
         * @param name
         *     A short name.
         * @param description
         *     A description of the request.
         * @param amount
         *     The cost of the item.
         * @param url
         *     An optional URL with more information.
         * @return a future
         */
        make: (state, name, description, amount, url) => module.post(
            "request/{}".format(state.me.uid), {
                name,
                description,
                amount,
                url: url ? url : undefined,
            }),
    },

    /**
     * Retrieves an account overview.
     *
     * @param state
     *     The application state.
     * @return a future
     */
    overview: (state) => module.get(
        "overview/{}".format(state.family.uid))
        .then(async r => {
            const family = r.family;
            family.members = listToMap(r.members, "uid");
            const me = family.members[state.me.uid];
            const balance = r.balances[state.me.uid];
            await state.update({
                me: {
                    uid: me.uid,
                    name: me.name,
                    email: me.email,
                    role: me.role,
                },
                account: {
                    balance: balance !== undefined
                        ? {
                            num: balance,
                            str: (
                                r.currency.format[0]
                                + balance
                                + r.currency.format[1]),
                        }
                        : {},
                },
                currency: r.currency,
                family,
            }).store();
            return r;
        }),

    session: {
        /**
         * Introspects the current session.
         *
         * If the request is successful, the state is updated.
         *
         * @param state
         *     The application state.
         * @return a future
         */
        introspect: (state) => module.get(
            "session/introspect")
            .then(async r => {
                const me = r.user;
                await state.update({
                    me: {
                        uid: me.uid,
                        name: me.name,
                        email: me.email,
                        role: me.role,
                    },
                    family: {
                        uid: me.family_uid,
                    },
                }).store();
                return r;
            }),

        /**
         * Performs log in.
         *
         * If the request is successful, the state is updated.
         *
         * @param state
         *     The application state.
         * @param email
         *     The user email address.
         * @param password
         *     The user password.
         * @return a future
         */
        login: (state, email, password) => module.post(
            "session/login", {
                email,
                password,
                email,
                password,
            })
            .then(async r => {
                const me = r.user;
                await state.clear();
                await state.update({
                    me: {
                        uid: me.uid,
                        name: me.name,
                        email: me.email,
                        role: me.role,
                    },
                    family: {
                        uid: me.family_uid,
                    },
                }).store();
                return r;
            }),

        /**
         * Performs log out.
         *
         * If the request is successful, the state is updated.
         *
         * @param state
         *     The application state.
         * @return a future
         */
        logout: (state) => module.post(
            "session/logout")
            .then(async r => {
                await state.clear();
                return r;
            }),
    },

    family: {
        /**
         * Adds a new family member.
         *
         * @param state
         *     The application state.
         * @param name
         *     The name of the user.
         * @param role
         *     The user role.
         * @param email
         *     The email address.
         * @param password
         *     The user password.
         * @param allowance
         *     The child allowance on the format `{amount, schedule}`. This is
         *     allowed only when adding a child.
         * @return a future
         */
        add: (state, name, role, email, password, allowance) => module.post(
            "family/{}".format(state.family.uid), {
                user: { name, role, email },
                allowance,
                password,
            })
            .then(r => {
                const user = r.user;
                const members = listToMap([user], "uid");
                state.update({
                    family: {
                        members,
                    },
                }).store();
                return r;
            }),

        /**
         * Registers a new family.
         *
         * @param state
         *     The application state.
         * @param userName
         *     The name of the main family user.
         * @param familyName
         *     The name of the family.
         * @param email
         *     The email address of the main family user.
         * @param password
         *     The user password.
         * @return a future
         */
        register: (state, userName, familyName, email, password) => module.post(
            "family", {
                family: {
                    name: familyName,
                },
                user: {
                    email,
                    name: userName,
                },
                password: password,
            })
            .then(r => {
                const me = r.user;
                state.update({
                    me: {
                        uid: me.uid,
                        name: me.name,
                        email: me.email,
                        role: me.role,
                    },
                    family: {
                        uid: me.family_uid,
                    },
                }).store();
                return r;
            }),

        /**
         * Removes a user from a family.
         *
         * @param state
         *     The application state.
         * @param user
         *     The unique ID of the user.
         * @return a future
         */
        remove: (state, user) => module.remove(
            "family/{}/{}".format(state.family.uid, user))
            .then(async r => {
                delete state.family.members[user];
                await state.store();
                return r;
            }),
    },

    /**
     * Retrieves information about the server.
     *
     * @param state
     *     The application state.
     */
    server: () => module.get("server"),

    /**
     * A wrapper for `fetch` with method `GET` taking `BASE_URL` into account.
     *
     * This function automatically parses the response as JSON.
     */
    get: (resource) => req(resource, {
        method: "GET",
    }),

    /**
     * A wrapper for `fetch` with method `DELETE` taking `BASE_URL` into account.
     *
     * This function automatically parses the response as JSON.
     */
    remove: (resource) => req(resource, {
        method: "DELETE",
    }),

    /**
     * A wrapper for `fetch` with method `POST` taking `BASE_URL` into account.
     *
     * This function automatically parses the response as JSON.
     */
    post: (resource, data) => req(resource, {
        method: "POST",
        body: JSON.stringify(data),
        headers: {
            "Content-Type": "application/json",
        },
    }),

    /**
     * A wrapper for `fetch` with method `PUT` taking `BASE_URL` into account.
     *
     * This function automatically parses the response as JSON.
     */
    put: (resource, data)  => req(resource, {
        method: "PUT",
        body: JSON.stringify(data),
        headers: {
            "Content-Type": "application/json",
        },
    }),
};

export default module;


/**
 * Converts a list to a mapping.
 *
 * The mapping is generated by reading the key as the property `key`.
 *
 * @param list
 *     The list to convert.
 */
const listToMap = (list, key) => {
    return list.reduce(
        (acc, i) => {
            acc[i[key]] = i;
            return acc;
        },
        {});
};


/**
 * A wrapper for `fetch` taking `BASE_URL` into account.
 *
 * This function automatically parses the response as JSON and handles
 * connection errors.
 *
 * @param resource
 *     The relative path.
 * @param init
 *     The initialisation value passed to `fetch`.
 */
const req = async (resource, init) => {
    return await fetch(BASE_URL + resource, init)
        .then((r) => {
            if (r.status >= 500) {
                throw r.statusText;
            } else {
                return r;
            }
        })
        .catch(DEFAULT_ERROR_HANDLER)
        .then(async r => {
            if (!r.ok) {
                throw r;
            } else if (r.headers.get("Content-Type") === "application/json") {
                return await r.json();
            } else {
                return await r.text();
            }
        });
};
