# Shared docs fragments

Markdown included into several pages with `<!--@include: @/_shared/...-->`. This folder is excluded from the site, so the files only appear where they're included.

- `options/`: one file per generator option, holding the description every tool shares. Each tool's configuration page adds the option's name, type, and default for that tool around the include. Write option names in camelCase and use absolute links.
