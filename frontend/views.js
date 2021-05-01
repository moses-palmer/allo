import api from "./api.js";
import login from "./views/login.js";
import make_request from "./views/make-request.js";
import overview from "./views/overview.js";
import register from "./views/register.js";
import request from "./views/request.js";

/**
 * The views of the application.
 *
 * The names correspond to classes added to the body for simplicity.
 */
const module = {
    "login": login,
    "make-request": make_request,
    "overview": overview,
    "register": register,
    "request": request,

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

        // Default to the overview
        return "overview";
    },
};

export default module;
