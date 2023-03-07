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

        const firstMember = Object.keys(state.family.members).length < 2;
        const cancel = view.doc.querySelector("#cancel");
        if (firstMember) {
            cancel.style.display = "none";
        }

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            const data = new FormData(form);
            ui.clearPasswords(form);

            await api.family.add(
                state,
                data.get("user-name"),
                data.get("user-role"),
                data.get("user-email"),
                data.get("password"),
                data.get("user-role") === "child"
                    ? {
                        amount: parseInt(data.get("allowance-amount")),
                        schedule: data.get("allowance-schedule"),
                    }
                    : undefined,
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
                        _("Failed to add member"),
                        _("This email address or name is already in use. "
                            + "Please provide a different email address or "
                            + "name."));
                    break;
                default:
                    await ui.message(
                        _("Failed to add member"),
                        _("Failed to add family member with message: {}")
                            .format(await e.text()));
                    break;
                }
            });
        });
    },
};
