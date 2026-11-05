Settings for individual formats, keyed by format:

| Format  | Field                 | Meaning                                                                                                                                   |
| ------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `svg`   | `centerVertically`    | Overrides the top-level `centerVertically`.                                                                                               |
| `svg`   | `fontId`              | The SVG font's `id` attribute. Defaults to the font name.                                                                                 |
| `svg`   | `metadata`            | Content of the SVG font's `<metadata>` element.                                                                                           |
| `svg`   | `optimizeOutput`      | Overrides the top-level `optimizeOutput`.                                                                                                 |
| `svg`   | `preserveAspectRatio` | Overrides the top-level `preserveAspectRatio`.                                                                                            |
| `ttf`   | `copyright`           | Copyright string in the font's `name` table.                                                                                              |
| `ttf`   | `description`         | Description string in the font's `name` table.                                                                                            |
| `ttf`   | `ts`                  | Creation and modification time, in Unix seconds. Set it for reproducible output; WOFF and WOFF2 carry it over.                            |
| `ttf`   | `url`                 | Manufacturer URL in the font's `name` table.                                                                                              |
| `ttf`   | `version`             | Version string in the font's `name` table.                                                                                                |
| `woff`  | `metadata`            | XML for the WOFF metadata block.                                                                                                          |
| `woff2` | `compressionQuality`  | Brotli quality from `0` (fastest, largest) to `11` (slowest, smallest), `11` by default. It only changes the file size, never the glyphs. |
