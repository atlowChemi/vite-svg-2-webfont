import { describe, expect, it } from 'vite-plus/test';
import { extractBlocks } from './blocks.ts';

const names = new Set(['ExampleTabs', 'ExampleCss']);
const extract = (source: string) => extractBlocks(source, names, i => `<!--block:${i}-->`).blocks;

describe('extractBlocks', () => {
    it('keeps a fenced Vue file whole inside a slot, closing tags and all', () => {
        const vue = ['```vue [Icon.vue]', '<template>', '    <i class="icon" />', '</template>', '```'].join('\n');
        const [block] = extract(
            [
                '<ExampleTabs>',
                '<template #config>',
                '',
                vue,
                '',
                '```ts [Icon.tsx]',
                'export {};',
                '```',
                '',
                '</template>',
                '<template #css>',
                '<ExampleCss example="basic" />',
                '</template>',
                '</ExampleTabs>',
            ].join('\n'),
        );
        expect(block!.slots.config).toBe(`${vue}\n\n\`\`\`ts [Icon.tsx]\nexport {};\n\`\`\``);
        expect(block!.slots.css).toBe('<ExampleCss example="basic" />');
    });

    it('nests plain <template> elements outside code', () => {
        const [block] = extract(['<ExampleTabs>', '<template #a>', '<template>', 'inner', '</template>', '</template>', '</ExampleTabs>'].join('\n'));
        expect(block!.slots.a).toBe('<template>\ninner\n</template>');
    });

    it('collects content outside named slots as the default slot', () => {
        const [block] = extract(['<ExampleTabs>', 'Loose text', '<template #a>', 'A', '</template>', '</ExampleTabs>'].join('\n'));
        expect(block!.slots).toEqual({ a: 'A', default: 'Loose text' });
    });

    it('evaluates bound props and reads plain ones', () => {
        const [block] = extract(`<ExampleCss example="basic" :highlight="['a', 'b']" data-x />`);
        expect(block!.props).toEqual({ example: 'basic', highlight: ['a', 'b'], dataX: true });
    });

    it('fails on a slot that never closes', () => {
        expect(() => extract(['<ExampleTabs>', '<template #a>', 'A', '</ExampleTabs>'].join('\n'))).toThrow('<template #a> is never closed');
    });
});
