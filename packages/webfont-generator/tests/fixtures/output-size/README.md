# Frozen output-size icons

`icons.json` contains the first 300 entries returned by
`Object.keys(iconSet.icons)` from `@iconify-json/simple-icons@1.2.92/icons.json`,
in their original order. That package reports Simple Icons version **16.27.1**.
Each entry preserves the original name and SVG body, with width and height
resolved using `icon.width ?? iconSet.width ?? 24` (and likewise for height).

Source: [Simple Icons Collaborators](https://github.com/simple-icons/simple-icons),
distributed through [Iconify](https://github.com/iconify/icon-sets).
The source package identifies the icon collection as
[CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
Brand names and logos remain subject to their respective trademark rights.

The deterministic output-size tests use this frozen corpus so their inline
snapshots measure generator changes against identical inputs. **Do not refresh
this file when updating Iconify dependencies.** The separate installed Iconify
compatibility tests exercise the installed packages without exact size assertions.

An intentional corpus change should preserve explicit ordering, document the new
source version and selection here, and include reviewed snapshot updates in the
same change. Keep the test's SVG wrappers, filenames, and generation options
stable unless deliberately changing the baseline.
