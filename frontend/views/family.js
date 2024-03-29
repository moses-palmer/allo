import { translate as _ } from "../translation.js";
import * as ui from "../ui.js";


export default {
    initialize: async (_state) => {},

    show: async (view, state) => {
        const memberRow = (state, m, template) => {
            const tr = template.content.cloneNode(true);
            const [name, email] = ui.managed(tr);
            name.innerText = m.uid === state.me.uid
                ? _("{} (me)").format(m.name)
                : m.name;
            name.addEventListener(
                "click",
                () => location.hash = `#user/${m.uid}`);
            email.innerText = m.email;
            return tr;
        };

        const table = (selector, all, mapper) => {
            const [rowTemplate, target] = ui.extractElement(view.doc, selector);
            all
                .forEach(t => target.appendChild(
                    mapper(state, t, rowTemplate)));
        };

        view.doc.querySelector("#add-member").addEventListener(
            "click",
            () => location.hash = state.server?.features.includes("email")
                ? "#invite"
                : "#add-member"
        );

        table(
            "#parents template",
            Object.values(state.family.members)
                .filter((m) => m.role === "parent"),
            memberRow);
        table(
            "#children template",
            Object.values(state.family.members)
                .filter((m) => m.role === "child"),
            memberRow);
    },
};
