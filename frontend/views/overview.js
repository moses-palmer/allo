import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: async (state) => {
        return await api.overview(state)
            .then((r) => {
                const children = (members) => members
                    .filter((member) => member.role === "child");
                if (children(Object.values(state.family.members))
                        .length === 0) {
                    throw "add-member";
                } else {
                    return r;
                }
            })
    },

    show: async (state, doc) => {
        const transactionRow = (state, t, template) => ui.transactionRow(
            state, template.content.cloneNode(true), t);

        const requestRow = (state, r, template) => ui.requestRow(
            state, template.content.cloneNode(true), r);

        const childTable = (selector, all, mapper) => {
            const [rowTemplate, target] = ui.extractElement(doc, selector);
            all
                .filter((t) => t.user_uid === state.me.uid)
                .forEach(t => target.appendChild(
                    mapper(state, t, rowTemplate)));
        };

        const parentTable = (selector, all, mapper, title, transformer) => {
            const [tableTemplate, tableTarget] = ui.extractElement(
                doc, selector);
            const children = Object.entries(state.family.members)
                .filter(([_, user]) => user.role === "child");
            children
                .forEach(([uid, child]) => {
                    const items = all.filter((t) => t.user_uid === uid);
                    const table = tableTemplate.content.cloneNode(true);
                    if (transformer) {
                        transformer(child, table);
                    }
                    const [rowTemplate, target] = ui.extractElement(
                        table, "template");
                    const [caption] = ui.managed(table);
                    caption.innerText = title(child);
                    if (items.length > 0) {
                        items.forEach(t => target.appendChild(
                            mapper(state, t, rowTemplate)));
                        table.querySelector("table").classList.remove("empty");
                    }
                    tableTarget.appendChild(table);
                });

            if (tableTarget.querySelectorAll("table").length === 0) {
                tableTarget.remove();
            }
        };

        if (state.me.role === "child") {
            childTable(
                "#my-transactions template",
                state.context.transactions,
                transactionRow);
            childTable(
                "#my-requests template",
                state.context.requests,
                requestRow);
        }

        if (state.me.role === "parent") {
            parentTable(
                "#family-transactions",
                state.context.transactions,
                transactionRow,
                (child) => child.name);
            parentTable(
                "#family-requests",
                state.context.requests,
                requestRow,
                (child) => _("{user} ({amount})").format({
                    user: child.name,
                    amount: ui.currency(
                        state, state.context.balances[child.uid]),
                }));
        }
    },
};
