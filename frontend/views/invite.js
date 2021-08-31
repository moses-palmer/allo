import api from "../api.js";
import { translate as _, languageCode } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: (_state) => {},

    show: async (state, doc) => {
        const form = doc.getElementById("form");

        let allowanceRequired = true;
        const targets = form.querySelectorAll("input[name ^= 'allowance-']");
        const update = (f) => f.setCustomValidity(
            (allowanceRequired && f.value.length == 0)
                ? "invalid"
                : "");
        targets.forEach(update);
        targets.forEach((f) => f.addEventListener(
            "input",
            (el) => update(el.target)));
        form.querySelectorAll("input[name = 'user-role']")
            .forEach((el) => el.addEventListener(
                "change",
                (el) => {
                    allowanceRequired = el.target.value === "child";
                    targets.forEach(update);
                }));

        if ((Object.keys(state.family.members).length
                + state.family.invitations.length) < 2) {
            doc.querySelector("#cancel").style.display = "none";
        }

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);

            await api.invitation.create(
                state,
                data.get("user-role"),
                data.get("user-name"),
                data.get("user-email"),
                data.get("user-role") === "child"
                    ? {
                        amount: parseInt(data.get("allowance-amount")),
                        schedule: data.get("allowance-schedule"),
                    }
                    : undefined,
                languageCode(),
            ).then(() => {
                if (firstMember) {
                    location.hash = "#";
                } else {
                    history.back();
                }
            })
            .catch(async (e) => {
                ui.applyState(state);

                switch (e.status) {
                case 409:
                    await ui.message(
                        _("Failed to invite member"),
                        _("This email address or name is already in use. "
                            + "Please provide a different email address or "
                            + "name."));
                    break;
                default:
                    await ui.message(
                        _("Failed to invite member"),
                        _("Failed to invite family member with message: {}")
                            .format(e.text));
                    break;
                }
            });
        });
    },
};
