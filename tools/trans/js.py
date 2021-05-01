import re

from io import IOBase
from typing import Sequence


#: The regular expression matching
TRANS_RE = re.compile(r'''(?mux)
    _N?\(\s*
    (
        # This is the first string, the one we capture
        "(\\|\"|[^"])*?"
        (?:\s* \+ "(\\|\"|[^"])*?")*
    )
    (?:
        # For the _N form, additional arguments are supported
        ,
        "(\\|\"|[^"])*?"
        (?:\s* \+ "(\\|\"|[^"])*?")*
    )*
    \s*\)''')


def load(source: IOBase) -> Sequence[str]:
    """Loads translatable strings from a Javascript file.

    This function finds all occurrences of the translation functions ``_`` and
    ``_N``, extracts its first argument and then performs ``eval`` on that
    expression.
    """
    data = source.read()

    for m in TRANS_RE.finditer(data):
        yield eval(compile('({})'.format(m.group(1)), '__code__', 'eval'))
