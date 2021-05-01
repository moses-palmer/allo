import json

from collections import OrderedDict
from io import IOBase
from typing import Mapping, Sequence


class Catalog:
    def __init__(self, code: str, texts: Mapping[str, str]):
        self._code = code
        self._texts = texts

    @classmethod
    def load(cls, source: IOBase):
        """Loads a catalogue from a JSON file.

        :param source: The source.

        :return: a catalogue

        :raises ValueError: if the data does not contain all required fields,
            or their values are incorrectly typed
        """
        data = json.load(source)
        code = data['code']
        if not isinstance(code, str):
            raise ValueError('code')
        texts = data['texts']
        if not isinstance(texts, dict):
            raise ValueError('texts')
        if not all(
                isinstance(k, str) and isinstance(v, (str, dict))
                for (k, v) in texts.items()):
            raise ValueError('texts')

        return cls(code, texts)

    def store(self, target: IOBase):
        """Stores this catalog as a JSON file.

        The order of keys and texts is stable.

        :param target: The target.
        """
        texts = OrderedDict()
        texts.update(
            (k, self.texts[k])
            for k in sorted(
                key
                for key in self.texts.keys()
                if not bool(self.texts[key])))
        texts.update(
            (k, self.texts[k])
            for k in sorted(
                key
                for key in self.texts.keys()
                if bool(self.texts[key])))

        me = OrderedDict()
        me['code'] = self.code
        me['texts'] = texts

        json.dump(me, target, indent=4)

    def merge(self, texts: Sequence[str]) -> Mapping[str, str]:
        """Merges a new list of translatable strings with this catalogue.

        This method will update this catalogue by removing all items not
        present in ``texts``, and settings all items without a corresponding
        translation to ``''``.

        :param texts: The list of new texts.

        :return: a mapping of obsolete texts to their translations
        """
        obsolete = {
            k: v
            for (k, v) in self.texts.items()
            if k not in texts}

        self._texts = {
            k: self.texts.get(k, '')
            for k in texts}

        return obsolete

    @property
    def code(self) -> str:
        """The language code.
        """
        return self._code

    @property
    def texts(self) -> Mapping[str, str]:
        """The translated texts.

        Texts lacking translations are set to the empty string (``''``).
        """
        return self._texts
