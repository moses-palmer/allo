/**
 * Constructs a validator that ensures that fields match.
 *
 * @param checker
 *     A custom validation function for the values.
 * @param fields
 *     The fields to validate.
 */
export const matches = (checker, ...fields) => (el) => {
    const value = el.target.value;
    const valid = checker(value) && fields.every((f) => f.value === value);

    fields.forEach((f) => f.setCustomValidity(valid
        ? ""
        : "invalid"));
};


/**
 * A validator for passwords.
 *
 * This can be used with `matches`.
 *
 * @param val
 *     The password to validate.
 */
export const password = (val) => val.length > 6;
