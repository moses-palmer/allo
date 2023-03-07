import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as validators from "../validators.js";
import * as ui from "../ui.js";


/**
 * The various modes for this view.
 */
const MODES = [
    "invited"];


export default {
    initialize: async (state, invitation, mode) => await api.invitation.get(
            state, invitation)
        .then((r) => {
            if (MODES.includes(mode)) {
                ui.applyRole(r.invitation.role);
                return {mode, ...r};
            } else {
                throw `join/${invitation}/${MODES[0]}`;
            }
        }),

    show: async (view, state) => {
        const form = view.doc.getElementById("form");

        ui.applyMode(view.doc, MODES, view.context.mode);

        const passwords = form
            .querySelectorAll("input[type = password]");
        passwords.forEach((password) => password.addEventListener(
            "input",
            validators.matches(validators.password, ...passwords)));

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);
            ui.clearPasswords(form);

            await api.invitation.accept(
                state,
                view.context.invitation.uid,
                data.get("password"),
            ).then(() => location.hash = "#login")
            .catch(async (e) => {
                ui.applyState(state);

                switch (e.status) {
                case 409:
                    await ui.message(
                        _("Failed to join family"),
                        _("You appear to have already joined this family."));
                    break;
                default:
                    await ui.message(
                        _("Failed to join family"),
                        _("Failed to join family with message: {}")
                            .format(await e.text()));
                    break;
                }
            });
        });
    },
};
