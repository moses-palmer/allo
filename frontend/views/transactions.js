import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


/**
 * The number of rows in the transaction table.
 */
const ROWS = 10;


export default {
    initialize: async (state, user_uid) => {
        const user = state.family.members[user_uid];
        return {user};
    },

    show: async (state, doc) => {
        const [earlier, later] = ui.managed(doc);

        const [rowTemplate, target] = ui.extractElement(
            doc, "#transactions template");
        for (let i = 0; i < ROWS; i++) {
            target.appendChild(ui.transactionRow(
                state, rowTemplate.content.cloneNode(true)));
        }

        let offset = 0;
        const update = async (d) => {
            const rows = target.querySelectorAll("tr");
            const limit = rows.length;
            try {
                const next = offset + d < 0
                    ? 0
                    : offset + d;
                const {transactions} = await api.transaction.list(
                    state, state.context.user.uid, next, limit);

                earlier.disabled = transactions.length < limit;
                later.disabled = next === 0;

                rows.forEach((tr, i) => ui.transactionRow(
                    state, tr, transactions[i]));

                offset = next;
            } catch (e) {
                switch (e.status) {
                default:
                    await ui.message(
                        _("Failed to list"),
                        _("Failed to list events with message: {}")
                            .format(e));
                    break;
                }
            }
        };

        earlier.addEventListener("click", () => update(ROWS));
        later.addEventListener("click", () => update(-ROWS));

        await update(0);
    },
};
