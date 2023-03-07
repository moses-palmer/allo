import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


/**
 * The timeout to update the allowance after changing the fields.
 */
const ALLOWANCE_UPDATE_TIMEOUT = 2000;


export default {
    initialize: async (state, user_uid) => {
        return await api.user.get(state, user_uid);
    },

    show: async (view, state) => {
        const scheduleOption = () => {
            const value = view.context.allowance?.schedule.toLowerCase();
            return parentAllowance.querySelector(`option[value ="${value}"]`);
        };
        const queryRemove = async () => {
            while (true) {
                const body = removeTemplate.content.cloneNode(true);
                const [name] = ui.managed(body);
                name.addEventListener(
                    "input",
                    () => name.setCustomValidity(
                        (name.value === view.context.user.name)
                            ? ""
                            : "invalid"));
                const r =  await ui.show(
                    body,
                    [
                        {name: "yes", text: _("Yes"), classes: ["remove"]},
                        {name: "no", text: _("No"), classes: []},
                    ]);
                if (r === "yes" && name.value === view.context.user.name) {
                    return true;
                } else if (r === "no") {
                    return false;
                } else {
                    continue;
                }
            }
        };

        const [role, childAllowance, _schedule, parentAllowance, remove] =
            ui.managed(view.doc);
        const removeTemplate = view.doc.querySelector("template.remove");

        // Users cannot remove themselves
        if (view.context.user.uid === state.me.uid) {
            remove.style.display = "none";
        }

        // If the user does not have any allowance, hide the display and form
        if (!view.context.allowance) {
            childAllowance.style.display = "none";
            parentAllowance.style.display = "none";
        } else {
            scheduleOption().selected = true;

            parentAllowance.querySelectorAll(".allowance-input")
                .forEach((el) => {
                    const onTimeout = async () => {
                        try {
                            const data = new FormData(parentAllowance);
                            await api.user.allowance(
                                state,
                                view.context.user.uid,
                                view.context.allowance.uid,
                                parseInt(data.get("allowance-amount")),
                                data.get("allowance-schedule"));
                        } catch (e) {
                            // Ignore
                        }
                    };

                    let timer;
                    timer = el.addEventListener("input", () => {
                        clearTimeout(timer);
                        timer = setTimeout(onTimeout, ALLOWANCE_UPDATE_TIMEOUT)
                    });
                });
        }

        if (view.context.user.role === "child") {
            role.innerText = _("{user} is a child in the {family} family.")
                .format({
                    user: view.context.user.name,
                    family: state.family.name,
                });
        } else if (view.context.user.role === "parent") {
            role.innerText = _("{user} is a parent in the {family} family.")
                .format({
                    user: view.context.user.name,
                    family: state.family.name,
                });
        }

        if (state.me.role === "child") {
            const [schedule] = ui.managed(view.doc);
            schedule.innerText = scheduleOption()?.innerText;
        }

        remove.querySelector("input").addEventListener("click", async () => {
            if (await queryRemove()) {
                try {
                    await api.family.remove(state, view.context.user.uid);
                    history.back();
                } catch (e) {
                    switch (e.status) {
                    default:
                        await ui.message(
                            _("Failed to remove"),
                            _("Failed to remove member with message: {}")
                                .format(e));
                        break;
                    }
                }
            }
        });
    },
};
