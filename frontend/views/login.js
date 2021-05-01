import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: (_state) => {},

    show: async (state, doc) => {
        const form = doc.getElementById("form");

        form.querySelector("input[type = email]").value = state.me?.email || "";

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);
            ui.clearPasswords(form);

            try {
                await api.session.login(
                    state,
                    data.get("email"),
                    data.get("password"));
                location.hash = "#overview";
            } catch(e) {
                ui.applyState(state);

                switch (e.status) {
                case 401:
                    await ui.message(
                        _("Login failed"),
                        _("Invalid email or password."));
                    break;
                default:
                    await ui.message(
                        _("Login failed"),
                        _("Failed to log in with message: {}")
                            .format(e));
                    break;
                }
            }
        });
    },
};
