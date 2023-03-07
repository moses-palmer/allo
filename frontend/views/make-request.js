import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: async (_state) => {},

    show: async (view, state) => {
        const form = view.doc.getElementById("form");

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);

            try {
                await api.request.make(
                    state,
                    data.get("name"),
                    data.get("description"),
                    parseInt(data.get("amount")),
                    data.get("url"));
                ui.applyState(state);
                location.hash = "#overview";
            } catch(e) {
                ui.applyState(state);

                switch (e.status) {
                default:
                    await ui.message(
                        _("Failed to wish"),
                        _("Failed to wish with message: {}")
                            .format(await e.text()));
                    break;
                }
            }
        });
    },
};
