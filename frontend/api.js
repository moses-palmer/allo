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
