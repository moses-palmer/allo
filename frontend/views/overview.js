import api from "../api.js";
import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: async (state) => {
        return await api.overview(state)
            .then((r) => {
                const children = (members) => members
                    .filter((member) => member.role === "child");
                if ((children(Object.values(state.family.members)).length
                        + children(state.family.invitations).length) === 0) {
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

        const invitationsTable = (selector, invitations) => {
            const [invitationRowTemplate, tableTarget] = ui.extractElement(
                doc, selector);
            invitations.forEach((i) => {
                const el = invitationRowTemplate.content.cloneNode(true);
                const [name, email] = ui.managed(el);
                name.innerText = i.name;
                email.innerText = i.email;
                tableTarget.appendChild(el);
            });
            if (state.family.invitations.length === 0) {
                ui.removeParent(tableTarget, "SECTION");
            }
        };

        const giftTemplate = doc.querySelector(".gift.template").content;
        const gift = async (state, child) => {
            const body = giftTemplate.cloneNode(true);
            const [name, currencyPre, amount, currencyPost, description] =
                ui.managed(body);
            name.innerText = _("Gift for {}").format(child.name);
            currencyPre.innerText = state.currency.format[0];
            currencyPost.innerText = state.currency.format[1];
            const r = await ui.show(
                body,
                [
                    {name: "yes", text: _("Give gift")},
                    {name: "no", text: _("Cancel"), classes: ["cancel"]},
                ]);
            switch (r) {
            case "yes":
                return {
                    amount: parseInt(amount.value),
                    description: description.value,
                }
            case "no":
                return undefined;
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
                (child) => child.name,
                (child, table) => {
                    const [_name, link] = ui.managed(table);
                    link.innerText = _("See all for {}")
                            .format(child.name);
                    link.href = `#transactions.${child.uid}`;
                });
            parentTable(
                "#family-requests",
                state.context.requests,
                requestRow,
                (child) => _("{user} ({amount})").format({
                    user: child.name,
                    amount: ui.currency(
                        state, state.context.balances[child.uid]),
                }),
                (child, table) => {
                    const [_name, link] = ui.managed(table);
                    link.innerText = _("Give {} a gift!")
                        .format(child.name);
                    link.addEventListener(
                        "click",
                        async () => {
                            gift(state, child)
                                .then((q) => {
                                    if (q != undefined) {
                                        return api.transaction.create(
                                            state, child.uid, "gift", q.amount,
                                            q.description);
                                    }
                                })
                                .then((r) => {
                                    if (r !== undefined) {
                                        ui.update(state);
                                    }
                                });
                        });
                });
            invitationsTable(
                "#family-invitations-row",
               state.family.invitations);
        }
    },
};
