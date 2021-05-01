import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: async (state, user_uid, request_uid) => {
        const {request} = await api.request.get(state, user_uid, request_uid);
        const user = state.family.members[request.user_uid];
        return {request, user};
    },

    show: async (state, doc) => {
        const queryRemove = async () => await ui.show(
            declineTemplate.content.cloneNode(true),
            [
                {name: "yes", text: _("Yes")},
                {name: "no", text: _("No"), classes: ["cancel"]},
            ]) === "yes";

        const queryGrant = async () => {
            const body = grantTemplate.content.cloneNode(true);
            const [amount] = ui.managed(body);
            const r = await ui.show(
                body,
                [
                    {name: "yes", text: _("Yes")},
                    {name: "no", text: _("No"), classes: ["cancel"]},
                ]);
            switch (r) {
            case "yes":
                return parseInt(amount.value);
            case "no":
                return undefined;
            }
        };

        const [link, iframe, remove, grant] = ui.managed(doc);
        const declineTemplate = doc.querySelector("template.decline");
        const grantTemplate = doc.querySelector("template.grant");

        if (!state.context.request.url) {
            link.style.display = "none";
            iframe.style.display = "none";
        }

        remove.addEventListener("click", async () => {
            if (await queryRemove()) {
                try {
                    await api.request.decline(
                        state,
                        state.context.request.user_uid,
                        state.context.request.uid);
                    location.hash = "#overview";
                } catch(e) {
                    ui.applyState(state);

                    switch (e.status) {
                    case 404:
                        await ui.message(
                            _("Failed to decline wish"),
                            _("The wish no longer exists."));
                        location.hash = "#overview";
                        break;
                    default:
                        await ui.message(
                            _("Failed to decline wish"),
                            _("Failed to decline wish with message: {}")
                                .format(e));
                        break;
                    }
                }
            }
        });

        grant.addEventListener("click", async () => {
            const cost = await queryGrant();
            if (cost !== undefined) {
                try {
                    await api.request.grant(
                        state,
                        state.context.request.user_uid,
                        state.context.request.uid,
                        cost);
                    location.hash = "#overview";
                } catch(e) {
                    ui.applyState(state);

                    switch (e.status) {
                    case 404:
                        await ui.message(
                            _("Failed to grant wish"),
                            _("The wish no longer exists."));
                        location.hash = "#overview";
                        break;
                    default:
                        await ui.message(
                            _("Failed to grant wish"),
                            _("Failed to grant wish with message: {}")
                                .format(e));
                        break;
                    }
                }
            }
        });
    },
};
