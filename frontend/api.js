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

    session: {
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
