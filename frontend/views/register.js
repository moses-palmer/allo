import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as validators from "../validators.js";
import * as ui from "../ui.js";


export default {
    initialize: (_state) => {},

    show: async (view, state) => {
        const form = view.doc.getElementById("form");

        const passwords = form
            .querySelectorAll("input[type = password]");
        passwords.forEach((password) => password.addEventListener(
            "input",
            validators.matches(validators.password, ...passwords)));

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);
            ui.clearPasswords(form);

            try {
                await api.family.register(
                    state,
                    data.get("user-name"),
                    data.get("family-name"),
                    data.get("user-email"),
                    data.get("password"));
                location.hash = "#login";
            } catch(e) {
                ui.applyState(state);

                switch (e.status) {
                case 409:
                    email.value = "";
                    await ui.message(
                        _("Failed to register"),
                        _("This email address is already in use. Please "
                            + "provide a different email address."));
                    break;
                default:
                    await ui.message(
                        _("Failed to register"),
                        _("Failed to register with message: {}")
                            .format(e));
                    break;
                }
            }
        });
    },
};
