/**
 * The attribute that signifies that an element should be translated.
 */
const TRANSLATION_ATTRIBUTE = "data-trans";

/**
 * The string used to split a language code into its parts.
 */
const LANGUAGE_SPLITTER = "-";

/**
 * The catalog of translations.
 */
let CATALOG = false;

/**
 * Translates a string into the current language.
 *
 * @param s
 *     The string to translate.
 * @return a translated string, or s if no translation exists
 */
export const translate = (s) => get(s) || s;


/**
 * Translates a plural string into the current language.
 *
 * @param s
 *     The string to translate for plural index 0.
 * @param cases
 *     The other plural strings of the original language, keyed by the value
 *     returned by `Intl.Pluralizer.select`.
 * @return a callable transforming a numeral to a translated string
 */
export const translateN = (s, cases) => {
    return n => {
        const i = CATALOG.pluralizer(n);
        const c = get(s);
        if (c.constructor === Object) {
            return c[i];
        } else {
            return cases[i];
        }
    };
};

/**
 * The language code of the language loaded.
 *
 * @return a string
 */
export const languageCode = () => CATALOG.code;


/**
 * Attempts to load a translation catalogue for a language.
 *
 * If not translation for the language exists, this method attempts to make
 * a less specific language code and retry if possible.
 *
 * @param language
 *     The language code.
 */
export const load = async (language) => {
    const catalog = await fetch(`translations/${language}.json`)
        .then(r => {
            if (!r.ok) {
                throw r;
            } else {
                return r.json();
            }
        })
        .catch(e => {
            const s = language.lastIndexOf(LANGUAGE_SPLITTER);
            if (s == -1) {
                throw e;
            } else {
                return load(language.substring(0, s));
            }
        });

    // Wrap all texts to sanitise lookups
    const texts = {}
    for (const [k, v] of Object.entries(catalog.texts)) {
        texts[wrap(k)] = v;
    }

    CATALOG = {
        code: catalog.code,
        plural: new Intl.PluralRules(catalog.code).select,
        texts,
    };
};


/**
 * Applies translations to an element and its children.
 *
 * @param doc
 *     The element to translate.
 */
export const apply = (doc) => {
    // Translate the text of all elements
    doc.querySelectorAll(`*[${TRANSLATION_ATTRIBUTE}]`).forEach((el) => {
        const source = el.attributes[TRANSLATION_ATTRIBUTE].value;
        if (source) {
            el.attributes[source].value = translate(
                el.attributes[source].value.trim().split(/\s+/).join(" "));
        } else {
            el.innerHTML = translate(
                el.innerHTML.trim().split(/\s+/).join(" "));
        }
    });

    // Template elements have their child elements in a separate document under
    // the content field
    doc.querySelectorAll("template").forEach((el) => apply(el.content));

    return doc;
};


/**
 * Gets the catalogue entry for a string.
 *
 * @param k
 *     The catalogue entry key.
 * @return the catalogue entry, or nothing if no catalogue is loaded
 */
const get = (k) => {
    if (CATALOG && CATALOG.texts) {
        return CATALOG.texts[wrap(k)];
    }
};


/**
 * Ensures that a string can be compared with the `innerHTML` of an element.
 *
 * This function creates an empty tag, sets its `innerHTML` to `s` and then
 * returns the generated `innerHTML`.
 *
 * @param s
 *     The string to wrap.
 */
const wrap = (s) => {
    const el = document.createElement("t");
    el.innerHTML = s.trim().replace(/\s+/g, " ");
    return el.innerHTML;
};
