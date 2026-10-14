import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

try {
    const result = await generateWebfonts({
        sources: [{ id: 'icon', name: 'icon', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
        types: ['woff2'],
    });
    try {
        const bytes = await result.font('woff2');
        if (!bytes) throw new Error('Font is missing');
        await new FontFace('built-font', bytes).load();
        document.body.dataset.state = 'passed';
    } finally {
        result.dispose();
    }
} catch (error) {
    document.body.dataset.state = String(error);
}
