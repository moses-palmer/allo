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
    },
};

export default module;
