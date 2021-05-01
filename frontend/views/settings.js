import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as notifications from "../notifications.js";
import * as ui from "../ui.js";


export default {
    initialize: async (_state) => {},

    show: async (state, doc) => {
        const [form, enableNotifications, logout] = ui.managed(doc);

        if (window.isSecureContext) {
            enableNotifications.checked = notifications.isEnabled(state);
            enableNotifications.addEventListener("change", () => {
                if (enableNotifications.checked) {
                    notifications.enable(state, (enabled) => {
                        enableNotifications.checked = enabled;
                    });
                } else {
                    enableNotifications.checked = notifications.disable(state);
                }
            });
        } else {
            form.style.display = "none";
        }

        logout.addEventListener("click", async () => {
            const r =  await ui.message(
                _("Log out"),
                _("Are you sure you want to log out?"),
                [
                    {name: "yes", text: _("Yes"), classes: ["remove"]},
                    {name: "no", text: _("No"), classes: []},
                ]);
            if (r === "yes") {
                try {
                    await api.session.logout(state);
                    ui.applyState(state);
                    location.hash = "#login";
                } catch (e) {
                    await ui.message(
                        _("Failed to log out"),
                        _("Are you logged in?"));
                }
            }
        });
    },
};
