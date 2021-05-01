import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as validators from "../validators.js";
import * as ui from "../ui.js";


export default {
    initialize: (_state) => {},

    show: async (state, doc) => {
        const form = doc.getElementById("form");

        const passwords = form
            .querySelectorAll("input[autocomplete = new-password]");
        passwords.forEach((password) => password.addEventListener(
            "input",
            validators.matches(validators.password, ...passwords)));

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);
            ui.clearPasswords(form);

            try {
                await api.session.password(
                    state,
                    data.get("current-password"),
                    data.get("new-password"));
                location.hash = "#login";
            } catch(e) {
                ui.applyState(state);

                switch (e.status) {
                case 403:
                    await ui.message(
                        _("Failed to change password"),
                        _("You provided an invalid current password."));
                    break;
                default:
                    await ui.message(
                        _("Failed to change password"),
                        _("Failed to change password with message: {}")
                            .format(e));
                    break;
                }
            }
        });
    },
};
