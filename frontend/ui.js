import VIEWS from "./views.js";
import { translate as _ } from "./translation.js";


/**
 * The selecor used to find the main element.
 */
const MAIN_SELECTOR = "#main";

/**
 * The selector used to find elements relating to the application state.
 */
const APP_SELECTOR = ".app";

/**
 * The element that contains messages.
 */
const MESSAGES_EL = "messages";

/**
 * The class used for templates
 */
const TEMPLATE_CLASS = "template";

/**
 * The message box class.
 */
const MESSAGE_BOX_CLASS = "message-box";

/**
 * The message box button template class.
 */
const BUTTON_CLASS = "button";

/**
 * The generic message class.
 */
const MESSAGE_CLASS = "message";

/**
 * The selector used to find managed elements.
 */
const MANAGED_ELS = ".managed";

/**
 * The class applied to the body when the application is loading.
 */
const LOADING_CLASS = "loading";

/**
 * The class applied to the body when loading has completed.
 */
const READY_CLASS = "ready";

/**
 * The class added to the message overlay when messages are active.
 */
const ACTIVE_CLASS = "active";

/**
 * The class added to the message overlay when messages are fading away.
 */
const FADING_CLASS = "fading";

/**
 * The classes added to the body for the various user roles.
 */
const ROLE_CLASSES = {
    "child": "role-child",
    "parent": "role-parent",
};

/**
 * The class added to the body if the user is logged in.
 */
const LOGGED_IN_CLASS = "logged-in";

/**
 * The handler called when the first view has been loaded.
 */
let ON_READY = async() => {};


/**
 * Selects a named template under the message box template.
 *
 * @param name
 *     The selector.
 */
const messageBoxTemplate = (name) => document.getElementById(MESSAGES_EL)
    .querySelector(`.${MESSAGE_BOX_CLASS}.${TEMPLATE_CLASS}`).content
    .querySelector(name);


/**
 * Sets the on-ready handler.
 */
export const onReady = (handler) => {
    ON_READY = handler;
};


/**
 * Updates the UI and displays the view based on the location hash.
 *
 * When a view has been successfully called, the `onReady` handler is called.
 */
export const update = async (state) => {
    // The view is encoded as the hash
    const path = location.hash.substring(1) || await VIEWS.calculate(state);
    const parts = path.split(".");
    const name = parts[0].replaceAll("_", "-");
    const args = parts.slice(1);
    const view = VIEWS[parts[0]];

    // If the hash signifies a real application view, enforce it
    if (view) {
        // Clear the body class list and remove the current view element
        document.body.classList.forEach(className => {
            const parts = className.split("-");
            parts.pop();
            if (VIEWS[parts.join("-")]) {
                document.body.classList.remove(className);
            }
        });
        document.querySelectorAll(APP_SELECTOR)
            .forEach(el => el.remove());

        // Apply the state and update the body class and children
        try {
            state.context = await view.initialize(state, ...args);
            applyState(state);
            const html = view.html.replaceAll(
                /\$\{([^}]+)\}/g,
                (_, path) => state.get(path));
            const doc = (new DOMParser).parseFromString(html, "text/html");
            await view.show(state, doc);
            if (document.body.classList.contains(LOADING_CLASS)) {
                document.body.classList.remove(LOADING_CLASS);
                await ON_READY();
                document.body.classList.add(READY_CLASS);
            }
            document.body.classList.add(`${name}-view`);
            document.querySelector(MAIN_SELECTOR).append(
                doc.querySelector(APP_SELECTOR));
        } catch (e) {
            if (typeof e === "string") {
                location.hash = "#" + e;
            } else if (location.hash.length > 1) {
                console.trace(e);
                location.hash = "#";
            } else {
                throw e;
            }
        }
    }
};


/**
 * Calculates the length, in milliseconds, of an animation.
 *
 * This function assumes that the CSS variable `--animation-tick` is a value in
 * seconds.
 */
export const animationTick = () => 1000 * parseFloat(
    getComputedStyle(document.documentElement)
        .getPropertyValue("--animation-tick"));


/**
 * Clears all inputs marked as password entries from a form.
 *
 * @param form
 *     The form element.
 */
export const clearPasswords = (form) => form
    .querySelectorAll("input[type = password")
    .forEach((el) => {
        el.value = "";
    });


/**
 * Selects all managed elements under an element.
 *
 * Managed elements are input or output elements managed from code.
 *
 * @param el
 *     The main element, created from a template.
 * @param classes
 *     Additional classes.
 */
export const managed = (el, classes) => el.querySelectorAll(
    MANAGED_ELS + (!!classes ? `.${classes}` : ""));


/**
 *
 * This will hide all managed elements with the additional classes in `modes`,
 * except those with the class `mode`.
 *
 * @param el
 *     The main element, created from a template.
 * @param modes
 *     A list of all modes.
 * @param mode
 *     The mode to apply.
 */
export const applyMode = (el, modes, mode) => modes
    .forEach((m) => managed(el, m)
        .forEach((e) => e.style.display = m === mode
            ? "initial"
            : "none"));


/**
 * Selects an element and removes it from the document.
 *
 * The return value is the the array `[element, parent]`.
 *
 * @param doc
 *     The document from which to select the element.
 * @param selector
 *     The selector.
 */
export const extractElement = (doc, selector) => {
    const el = doc.querySelector(selector);
    if (el !== null) {
        const parent = el.parentNode;
        parent.removeChild(el);
        return [el, parent];
    }
};


/**
 * Removes a parent of which `el` is a child.
 *
 * The first parent of type `type` is removed.
 *
 * @param el
 *     The element from which to start the search.
 * @param tag
 *     The tag name of the parent to remove. Since this is compared to the
 *     `tagName` attribute of elements, it should be an upper case string.
 * @return whether an element was removed
 */
export const removeParent = (el, tag) => {
    let e = el;
    while (e && e.tagName !== tag) {
        e = e.parentNode;
    }
    if (e && e.remove) {
        e.remove();
        return true;
    } else {
        return false;
    }
};


/**
 * Displays a simple message box.
 *
 * @param caption
 *     The message caption.
 * @param text
 *     The message text.
 * @param buttons, on the format described by `show`. If this is not
 *     specified, The single button _OK_ is used.
 */
export const message = async (caption, text, buttons) => {
    const messageEl = messageBoxTemplate(
        `.${MESSAGE_CLASS}.${TEMPLATE_CLASS}`)
        .content.cloneNode(true);
    const [captionEl, textEl] = managed(messageEl);
    captionEl.innerText = caption;
    textEl.innerText = text;
    return await show(
        messageEl,
        (buttons || [{text: _("OK")}]));
};


/**
 * Displays a  message.
 *
 * @param message
 *     An element containing the message body.
 * @param buttons
 *     A list of buttons described by the following keys:
 *     * `name` - The name of the button. This is also the return value of this
 *       function.
 *     * `text` - The button text.
 *     * `classes` - A list of classes to apply to the button.
 */
export const show = async (
        message,
        buttons,
    ) => await new Promise((resolve) => {
    const top = document.getElementById(MESSAGES_EL);

    const messageBox = messageBoxTemplate(
        `.${MESSAGE_BOX_CLASS}`)
        .cloneNode(true);
    const buttonTemplate = messageBoxTemplate(
        `.${MESSAGE_BOX_CLASS}.${BUTTON_CLASS}.${TEMPLATE_CLASS}`);
    const messageCount = () => top.querySelectorAll(
        `.${MESSAGE_BOX_CLASS}:not(.${TEMPLATE_CLASS})`).length;

    const [messageEl, buttonsEl] = managed(messageBox);
    messageEl.appendChild(message);

    (buttons ? buttons : [{text: _("OK")}]).forEach(({name, text, classes}) => {
        const el = buttonTemplate.content.cloneNode(true);
        const input = el.querySelector("input");
        input.value = text;
        input.className = classes ? classes.join(" ") : "";
        input.addEventListener("click", () => {
            messageBox.parentNode.removeChild(messageBox);
            if (messageCount() === 0) {
                top.addEventListener("animationend", () => {
                    top.classList.remove(FADING_CLASS);
                    if (messageCount() === 0) {
                        top.classList.remove(ACTIVE_CLASS);
                    }
                    resolve(name);
                }, {once: true, passive: true});
                top.classList.add(FADING_CLASS);
            } else {
                resolve(name);
            }
        });
        buttonsEl.appendChild(el);

    });

    top.appendChild(messageBox);
    top.classList.add(ACTIVE_CLASS);
    top.classList.remove(FADING_CLASS);
});


/**
 * Applies a role to the interface.
 *
 * @param role
 *     The role to apply.
 */
export const applyRole = (role) => {
    // Synchronise role classes
    for (const [r, cls] of Object.entries(ROLE_CLASSES)) {
        if (r === role) {
            document.body.classList.add(cls);
        } else {
            document.body.classList.remove(cls);
        }
    }
};


/**
 * Applies the state to the interface.
 *
 * @param state
 *     The state to apply.
 * @return the state
 */
export const applyState = (state) => {
    // Synchronise role classes
    applyRole(state.me?.role);

    if (!state.me?.uid) {
        document.body.classList.remove(LOGGED_IN_CLASS);
    } else {
        document.body.classList.add(LOGGED_IN_CLASS);
    }

    return state;
}

/**
 * Converts a numerical value to a currency string.
 *
 * @param state
 *     The application state.
 * @param amount
 *     The currency amount.
 */
export const currency = (state, amount) => (
    state.currency.format[0]
    + new Intl.NumberFormat().format(amount)
    + state.currency.format[1]);


/**
 * Converts a date to a date string.
 *
 * @param state
 *     The application state.
 * @param time
 *     The timestamp.
 */
export const date = (state, time) => new Intl.DateTimeFormat(
    "default", {dateStyle: "medium"}).format(time);


/**
 * Converts a date to a timestamp string.
 *
 * @param state
 *     The application state.
 * @param time
 *     The timestamp.
 */
export const timestamp = (state, time) => new Intl.DateTimeFormat(
    "default", {dateStyle: "medium", timeStyle: "short"}).format(time);


/**
 * Displays a transaction in a modal view.
 *
 * @param state
 *     The application state.
 * @param transaction
 *     The transaction to display.
 */
export const transaction = async (state, transaction, buttons) => {
    const body = messageBoxTemplate(".transaction.template")
        .content.cloneNode(true);
    const [description, kind, time, amount] =
        managed(body);
    switch (transaction.transaction_type) {
    case "allowance":
        description.innerText = _("Allowance on {}")
            .format(date(state, new Date(Date.parse(transaction.time))));
        kind.innerText = _("Allowance");
        break;
    case "gift":
        description.innerText = transaction.description;
        kind.innerText = _("Gift");
        break;
    case "request":
        description.innerText = transaction.description;
        kind.innerText = _("Fulfilled wish");
        break;
    }
    time.innerText = timestamp(state, new Date(Date.parse(transaction.time)));
    amount.innerText = currency(state, transaction.amount);
    return await show(
        body,
        [
            {text: _("Close"), classes: ["ok"]},
        ].concat(buttons ? buttons : []));
};


/**
 * Updates a table rows with a transaction description.
 *
 * The column order is expected to be description, amount. The description will
 * become clickable if a transaction is specified.
 *
 * @param state
 *     The application state.
 * @param tr
 *     The table row element.
 * @param t
 *     The transaction to display. If this is undefined, the row is cleared.
 */
export const transactionRow = (state, tr, t) => {
    const [description, amount] = managed(tr);
    if (t) {
        description.innerText = t.transaction_type == "allowance"
            ? _("Allowance")
            : t.description;
        description.onclick = () => transaction(state, t);
        amount.innerText = currency(state, t.amount);
    } else {
        description.innerHTML = "&nbsp;";
        description.onclick = () => {};
        amount.innerHTML = "&nbsp;";
    }
    return tr;
};


/**
 * Updates a table rows with a request description.
 *
 * The column order is expected to be description, amount. The description will
 * become clickable if a request is specified.
 *
 * @param state
 *     The applicaiton state.
 * @param tr
 *     The table row element.
 * @param r
 *     The request to display. If this is undefined, the row is cleared.
 */
export const requestRow = (state, tr, r) => {
    const [description, amount] = managed(tr);
    if (r) {
        description.innerText = r.name;
        description.onclick = () => location.href =
            `#request.${r.user_uid}.${r.uid}`;
        amount.innerText = currency(state, r.amount);
    } else {
        description.innerHTML = "&nbsp;";
        description.onclick = () => {};
        amount.innerHTML = "&nbsp;";
    }
    return tr;
};


export const notification = {
    /**
     * Parses a notification from the backend.
     *
     * The returned value has the keys `action`, which is the action to take if
     * the displayed notification is clicked, `title`, which is the
     * notification title, and `body`, which is the notification body.
     *
     * If the event is unknown or should not be handled, nothing is returned.
     *
     * @param state
     *     The application state.
     * @param e
     *     The event to parse.
     * @return a description of the event.
     */
    parse: (state, e) => {
        switch (e.type) {
        case "AllowanceUpdated":
            return {
                action: () => {},
                title: _("Allowance changed"),
                body: e.allowance.user_uid === state.me.uid
                    ? _("Your allowance was changed to {amount} by {parent}!")
                        .format({
                            amount: currency(state, e.allowance.amount),
                            parent: state.family.members[e.by].name,
                        })
                    : _("{parent} updated {child}s allowance.")
                        .format({
                            parent: state.family.members[e.by].name,
                            child: state.family.members[
                                e.allowance.user_uid].name,
                        }),
            };
        case "FamilyMemberInvited":
            return {
                action: () => {},
                title: _("Family member invited"),
                body: _("A new member was invited to your family: {user}!")
                    .format({
                        user: e.user.name,
                    }),
            };
        case "FamilyMemberAdded":
            return {
                action: () => location.hash = "#user"
                    + "." + e.user.uid,
                title: _("Family member added"),
                body: _("A new member was added to your family: {user}!")
                    .format({
                        user: e.user.name,
                    }),
            };
        case "FamilyMemberRemoved":
            return {
                action: () => location.hash = "#family",
                title: _("Family member remove"),
                body: _("A member was removed from your family: {user}!")
                    .format({
                        user: e.user.name,
                    }),
            };
        case "RequestCreated":
            return {
                action: () => location.hash = "#request"
                    + "." + e.request.user_uid
                    + "." + e.request.uid,
                title: _("Wish made"),
                body: _("{user} has made a wish: {wish}.")
                    .format({
                        user: state.family.members[e.request.user_uid].name,
                        wish: e.request.name,
                    }),
            };
        case "RequestGranted":
            return {
                action: () => {},
                title: _("Your wish has been granted!"),
                body: _("{parent} granted your wish: {wish}.")
                    .format({
                        parent: state.family.members[e.by].name,
                        wish: e.request.name,
                    }),
            };
        case "RequestDeclined":
            return {
                action: () => {},
                title: _("Your wish has been declined"),
                body: _("{parent} declined your wish: {wish}.")
                    .format({
                        parent: state.family.members[e.by].name,
                        wish: e.request.name,
                    }),
            };
        }
    },


    /**
     * Displays a notification.
     *
     * @param state
     *     The applicaiton state.
     * @param event
     *     The notification event to display.
     */
    show: async (state, event) => {
        if (state.notifications.enabled !== true) {
            return;
        }

        try {
            const parsed = notification.parse(state, event);
            if (parsed !== undefined) {
                const {action, title, body} = parsed;
                try {
                    await new Promise((resolve, reject) => {
                        const notification = new Notification(title, {body});
                        notification.onclick = resolve;
                        notification.onclose = reject;
                        notification.onerror = reject;
                    });
                    action();
                } catch (e) {
                    // Ignore
                }
            }
        } catch (e) {
            // Ignore rejected notifications and unknown events
            console.log(`Failed to handle ${JSON.stringify(event)}: ${e}`);
        }
    },
};
