import api from "./api.js";
import add_member from "./views/add-member.js";
import change_password from "./views/change-password.js";
import family from "./views/family.js";
import invite from "./views/invite.js";
import join from "./views/join.js";
import login from "./views/login.js";
import make_request from "./views/make-request.js";
import overview from "./views/overview.js";
import register from "./views/register.js";
import request from "./views/request.js";
import settings from "./views/settings.js";
import transactions from "./views/transactions.js";
import user from "./views/user.js";

/**
 * The views of the application.
 *
 * The names correspond to classes added to the body for simplicity.
 */
const module = {
    "add-member": add_member,
    "change-password": change_password,
    "family": family,
    "invite": invite,
    "join": join,
    "login": login,
    "make-request": make_request,
    "overview": overview,
    "register": register,
    "request": request,
    "settings": settings,
    "transactions": transactions,
    "user": user,

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
            return module["register"];
        }

        // Check whether we are logged in
        try {
            await api.session.introspect(state);
        } catch (e) {
            return module["login"];
        }

        // Default to the overview
        return module["overview"];
    },
};

export default module;
