import argparse
import difflib
import os
import sys

from typing import Sequence, Set, Tuple

from . import html, js
from .catalog import Catalog


#: Extractors of translatable strings for supported file extensions.
TEXT_EXTRACTORS = {
    'html': html.load,
    'js': js.load}

#: The threshold for similarity for changed texts.
SUGGESTION_THRESHOLD = 0.7


def main(catalog_directory: str, source_directory: str):
    catalogs = dict(load_catalogs(
        os.path.join(catalog_directory, p)
        for p in os.listdir(catalog_directory)
        if p.endswith('.json')))

    texts = set(load_texts(
        os.path.join(dirpath, filename)
        for dirpath, dirnames, filenames in os.walk(source_directory)
        for filename in filenames))

    for (path, catalog) in catalogs.items():
        obsolete = catalog.merge(texts)

        with open(path, 'w', encoding='utf-8') as f:
            catalog.store(f)

        if any(not v for v in catalog.texts.values()):
            print('\033[1mTexts missing translations for {}:\033[0m'.format(
                catalog.code))
            for text in sorted(k for (k, v) in catalog.texts.items() if not v):
                print('• {}'.format(text))
                suggestions = [
                    suggestion
                    for (w, suggestion) in sorted(
                        (
                            difflib.SequenceMatcher(None, text, k).ratio(),
                            v)
                        for (k, v) in obsolete.items()
                        if isinstance(v, str))
                    if w >= SUGGESTION_THRESHOLD]
                if suggestions:
                    print('  \033[1mSuggestions:\033[0m')
                    for suggestion in suggestions:
                        print('  • \033[0;32m{}\033[0m'.format(suggestion))



def load_catalogs(paths: Sequence[str]) -> Sequence[Tuple[str, Catalog]]:
    """Loads catalogue files from the paths given.

    If any file fails to load, the application is terminated.

    :param paths: The paths to load.

    :return: a mapping from path to catalogue
    """
    for path in paths:
        with open(path, encoding='utf-8') as f:
            try:
                yield (path, Catalog.load(f))
            except ValueError as e:
                sys.stderr.write(
                    'Failed to load {}: {}\n'.format(path, e))
                sys.exit(1)


def load_texts(paths: Sequence[str]) -> Set[str]:
    """Loads texts from the paths given.

    If any file fails to load, the application is terminated.

    :param paths: The paths to load.

    :return: a set of translatable texts
    """
    for path in paths:
        ext = path.rsplit('.', 1)[-1]
        if ext in TEXT_EXTRACTORS:
            with open(path, encoding='utf-8') as f:
                try:
                    yield from TEXT_EXTRACTORS[ext](f)
                except ValueError as e:
                    sys.stderr.write(
                        'Failed to load {}: {}\n'.format(path, e))
                    sys.exit(1)


parser = argparse.ArgumentParser(
    description='Extracts translatable strings from source files and merges '
    'them with translation catalogues.')

parser.add_argument(
    '--catalog-directory',
    help='The directory containing catalogue files.',
    required=True)

parser.add_argument(
    '--source-directory',
    help='The directory containing source files.',
    required=True)


main(**vars(parser.parse_args()))
