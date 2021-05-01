import api from "./api.js";
import login from "./views/login.js";
import register from "./views/register.js";

/**
 * The views of the application.
 *
 * The names correspond to classes added to the body for simplicity.
 */
const module = {
    "login": login,
    "register": register,

    /**
     * Attempts to calculate the next view given an application state.
     *
     * @param state
     *     The current application state.
     */
    calculate: async (state) => {
        // If we have no cached email address, we assume the user has not yet
        // registered
        if (state.me.email.length === 0) {
            return "register";
        }

        // Check whether we are logged in
        try {
            await api.session.introspect(state);
        } catch (e) {
            return "login";
        }
    },
};

export default module;
