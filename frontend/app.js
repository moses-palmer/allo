import VIEWS from "./views.js";
import api from "./api.js";
import connect from "./db.js";
import * as State from "./state.js";
import * as translation from "./translation.js";
import { translate as _ } from "./translation.js";
import * as ui from "./ui.js";


/**
 * Formats a string with replacement tokens.
 *
 * A replacement token is any string enclosed in brackets (`"{"` and `"}"`).
 * Use `"\"` to escape a token.
 *
 * The last argument to this method is the token map. This is an object where
 * the keys are tokens and the values their replacements.
 *
 * Empty tokens are positional tokens: instead of being looked up in `tokens`,
 * their index is mapped to the arguments to this method.
 */
String.prototype.format = function() {
    const tokens = arguments[arguments.length - 1];
    const max = arguments.length;
    let i = 0;
    return this.replaceAll(
        /\\\{|{([^}]*)}/g,
        (m, n) => {
            if (m === "\\{") {
                return "{";
            } else if (!n && i < max) {
                return arguments[i++];
            } else if (n in tokens) {
                return tokens[n];
            } else {
                throw `unknown token: ${!!n ? n : "[" + (i + 1) + "]"}`;
            }
        },
    );
};


/**
 * The selector used to find the logotype element.
 */
const LOGO_SELECTOR = "#logo";


const load = async () => {
    const errorManager = async () => {
        try {
            console.error(e, e.stack);
        } catch (e) {}

        if (e.reason !== "connection") {
            const title = _("An unexpected error occurred");
            const message = _("The error message is: {}").format(
                e.message && e.filename
                    ? `${e.message}; ${e.filename}:${e.lineno}`
                    : e.reason);

            const response = await ui.message(
                title,
                message,
                [
                    {name: "ignore", text: _("Ignore"), classes: ["cancel"]},
                    {name: "reload", text: _("Reload"), classes: ["ok"]},
                ]);
            if (response === "reload") {
                location.reload();
            }
        }
    };
    window.addEventListener("error", errorManager);
    window.addEventListener("unhandledrejection", errorManager);

    const db = await connect();
    const state = await State.load(
        async () => {
            const result = await db.load() || {};
            result.server = await api.server();
            return result;
        },
        async (v) => await db.store(v),
        async () => await db.clear());

    // Load translations
    try {
        const lang = new URLSearchParams(location.search).get("lang")
            ?? navigator.language.toLowerCase();
        await translation.load(lang);
        translation.apply(document.body);
    } catch (e) {
        // No translation available
    }

    // Load and translate all views
    await Promise.all(Object.entries(VIEWS)
        .filter(([_, view]) => typeof view !== "function")
        .map(async ([name, view]) => {
            view.html = (await fetch(`./views/${name}.html`)
                .then(r => r.text())
                .then(s => (new window.DOMParser()).parseFromString(
                    s,
                    "text/html"))
                .then(translation.apply))
                .body.innerHTML;
        }));

    api.defaultErrorHandler(errorManager);

    window.addEventListener("hashchange", (e) => {
        e.preventDefault();
        ui.update(state);
    });

    ui.onReady(async () => new Promise(resolve => setTimeout(() => {
        // Clip the logotype
        const logo = document.querySelector(LOGO_SELECTOR);
        const style = getComputedStyle(document.documentElement);
        logo.viewBox.baseVal.width = style.getPropertyValue(
            "--logo-target-viewbox-width");
        logo.style.width = style.getPropertyValue(
            "--logo-target-width");

        resolve();
    }, ui.animationTick() + 100)));

    await ui.update(state);
};


if ("serviceWorker" in navigator) {
    window.addEventListener("load", async () => {
        navigator.serviceWorker.register("./service.js", {
            scope: api.BASE_URL,
        });
        navigator.serviceWorker.addEventListener("message", async (event) => {
            const db = await connect();
            const state = await State.load(
                async () => await db.load(),
                async (v) => db.store(v),
                async () => db.clear());
        });
        await navigator.serviceWorker.ready;
        await load();
    });
} else {
    window.addEventListener("load", load);
}
