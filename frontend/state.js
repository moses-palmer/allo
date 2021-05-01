class State {
    /**
     * Constructs an application state from a set of values.
     *
     * Objects loaded from the backend are filtered so that only relevant
     * fields are used. Fields not present are set to default values.
     *
     * @param state
     *     The current state. Only relevant fields are used. Fields not present
     *     are set to their default values.
     * @param store
     *     An asynchronous function to store the state to the backend.
     * @param clear
     *     An asynchronous function to clear the state from the backend.
     */
    constructor(state, store, clear) {
        this._store = store;
        this._clear = clear;
        merge(TEMPLATE, this, state);
    }

    /**
     * Stores the application state to the database.
     */
    async store() {
        await this._store(copy(TEMPLATE, this));
        return this;
    }

    /**
     * Clears the current application state.
     */
    async clear() {
        // Do not clear the server information
        const server = this.server;
        Object.entries(TEMPLATE).forEach(([key, value]) => this[key] = value);
        this.server = server;
        await this._clear();
        return this;
    }

    /**
     * Reads an application state value given a path.
     *
     * @param path
     *     The value path. This is split on ".".
     */
    get(path) {
        return path.split(".").reduce(
            (acc, p) => p === "*" ? acc : acc ? acc[p] : null,
            this);
    }

    /**
     await load()* Updates the application state.
     *
     * @param values
     *     The values of the application state. Only relevant values are used.
     *     Undefined values are set to their defaults.
     */
    update(values) {
        merge(TEMPLATE, this, values);
        return this;
    }
}


/**
 * Loads the current application state.
 *
 * @param load
 *     An asynchronous function loading the state.
 * @param store
 *     An asynchronous function taking the current state and storing it.
 * @param clear
 *     An asynchronous function clearing the state.
 */
export const load = async (load, store, clear) => {
    try {
        return new State(await load(), store, clear);
    } catch (e) {
        console.log(`failed to load state: ${e}`);
        return new State(TEMPLATE, store, clear);
    }
};


/**
 * Determines whether a node is a leaf in a template.
 *
 * @param node
 *     The node to check.
 */
const leaf = (node) => node === null || node === undefined || !(false
    || (node.constructor === Array && node.length > 0)
    || (node.constructor === Object && Object.keys(node).length > 0));


/**
 * Extracts a copy of an object given a template.
 *
 * Items not present in `object` are copied from `template`. Items not present
 * in `template` are ignored.
 *
 * @param template
 *     The template object.
 * @param object
 *     The object to copy.
 */
const copy = (template, object) => Object.entries(template)
    .reduce((acc, [k, node]) => {
        if (leaf(node)) {
            acc[k] = (k in object) ? object[k] : node;
        } else {
            acc[k] = copy(node, k in object ? object[k] : node)
        }
        return acc;
    },
    template.constructor === Array ? [] : {});


/**
 * Merges one object into another using a template.
 *
 * Items not present in `target` or `source` are copied from `template`. Items
 * not present in `template` are ignored. Items present in `template` and
 * `target`, but absent from `source`, are kept.
 *
 * @param template
 *     The template object.
 * @param target
 *     The target object.
 * @param source
 *     The source object.
 */
const merge = (template, target, source) => Object.entries(template)
    .forEach(([k, node]) => {
        if (leaf(node)) {
            target[k] = (k in source)
                ? source[k]
                : (k in target) ? target[k] : node;
        } else {
            if (!(k in target)) {
                target[k] = {};
            }
            merge(node, target[k], (source && k in source) ? source[k] : {});
        }
    });


/**
 * The state template.
 */
const TEMPLATE = {
    server: {},
    family: {
        members: {},
        name: "",
        uid: "",
    },
    me: {
        email: "",
        name: "",
        role: "",
        uid: "",
    },
};
