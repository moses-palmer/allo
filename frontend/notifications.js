/**
 * Whether notifications are enabled.
 *
 * @param state
 *     The application state.
 */
export const isEnabled = (state) => Notification.permission === "granted"
    && state.notifications.enabled;


/**
 * Requests permission to use notifications.
 *
 * If permission has already been granted, no action is performed.
 *
 * This function will update the application state.
 *
 * @param state
 *     The application state.
 * @param callback
 *     A function called when the request has completed. It will be passed a
 *     flag signifying whether notifications are enabled.
 */
export const enable = (state, callback) => {
    const resolve = (permission) => {
        state.notifications.enabled = permission === "granted";
        state.store();
        callback(state.notifications.enabled);
    };

    if (!isEnabled(state)) {
        // Work around Safari not supporting the promise based notification API
        try {
            Notification.requestPermission().then(resolve);
        } catch (e) {
            Notification.requestPermission(resolve);
        }
    } else {
        callback(true);
    }
};


/**
 * Disables notifications.
 *
 * This function will update the application state.
 *
 * @param state
 *     The application state.
 */
export const disable = (state) => {
    state.notifications.enabled = false;
    state.store();

    return state.notifications.enabled;
};
