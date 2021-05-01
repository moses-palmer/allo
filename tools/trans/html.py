from io import IOBase
from lxml.html import document_fromstring
from typing import Sequence


#: The attribute used to mark an element for translation.
ATTR = 'data-trans'


def load(source: IOBase) -> Sequence[str]:
    """Loads translatable strings from an HTML 5 file.

    This function will find all elements with a ``data-trans`` attribute. If
    the attribute is empty, the text content, after normalisation by removing
    redundant whitespace, is used as translatable string, otherwise the value
    of the attribute indicated by the value is used.

    :param source: The source file.

    :returns: a sequence of translatable strings
    """
    for el in document_fromstring(source.read()).findall(
            './/*[@{}]'.format(ATTR)):
        attrib = el.attrib[ATTR]
        if attrib:
            if attrib in el.attrib:
                yield el.attrib[attrib]
            else:
                raise ValueError(
                    'unknown attribute {}'.format(attrib))
        else:
            yield ' '.join(el.text_content().split())
