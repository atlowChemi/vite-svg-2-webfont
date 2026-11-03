<template>
    <div v-html="svg" :class="props.class"></div>
</template>

<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { render } from './mermaid';

const props = defineProps({
    graph: {
        type: String,
        required: true,
    },
    id: {
        type: String,
        required: true,
    },
});

const svg = ref(null);
const code = ref(decodeURIComponent(props.graph));
const ctrlSymbol = ref(navigator.platform.includes('Mac') ? '⌘' : 'Ctrl');
const isFirefox = navigator.userAgent.toLowerCase().includes('firefox');
const contentEditable = ref(isFirefox ? 'true' : 'plaintext-only');

let mut = null;

const updateCode = event => {
    code.value = event.target.innerText;
};

onMounted(async () => {
    mut = new MutationObserver(() => renderChart());
    mut.observe(document.documentElement, { attributes: true });

    await renderChart();

    //refresh images on first render
    const hasImages = /<img([\w\W]+?)>/.exec(code.value)?.length > 0;
    if (hasImages)
        setTimeout(() => {
            let imgElements = document.getElementsByTagName('img');
            let imgs = Array.from(imgElements);
            if (imgs.length) {
                Promise.all(
                    Iterator.from(imgElements)
                        .filter(img => !img.complete)
                        .map(img => {
                            const { resolve, promise } = Promise.withResolvers();
                            img.addEventListener('load', resolve);
                            img.addEventListener('error', resolve);
                            return promise;
                        }),
                ).then(() => renderChart());
            }
        }, 100);
});

onUnmounted(() => mut.disconnect());

// Mermaid's base theme, colored from the site's tokens as they are for the current light or dark mode: nodes are drawn
// like the type case's compartments, and edges in the brand color.
const themeVariables = () => {
    const style = getComputedStyle(document.documentElement);
    const token = name => style.getPropertyValue(name).trim();
    return {
        fontFamily: token('--vp-font-family-base'),
        fontSize: '14px',
        background: token('--vp-c-bg'),
        primaryColor: token('--vp-c-bg-soft'),
        primaryBorderColor: token('--fonte-mold'),
        primaryTextColor: token('--vp-c-text-1'),
        textColor: token('--vp-c-text-1'),
        lineColor: token('--vp-c-brand-1'),
        edgeLabelBackground: token('--vp-c-bg'),
        clusterBkg: token('--vp-c-bg-alt'),
        clusterBorder: token('--vp-c-divider'),
        titleColor: token('--vp-c-text-2'),
    };
};

const renderChart = async () => {
    const mermaidConfig = {
        securityLevel: 'loose',
        startOnLoad: false,
        // SVG labels avoid HTML text being clipped by inherited page typography.
        htmlLabels: false,
        theme: 'base',
        themeVariables: themeVariables(),
    };
    let svgCode = await render(props.id, code.value, mermaidConfig);
    // This is a hack to force v-html to re-render, otherwise the diagram disappears
    // when **switching themes** or **reloading the page**.
    // The cause is that the diagram is deleted during rendering (out of Vue's knowledge).
    // Because svgCode does NOT change, v-html does not re-render.
    // This is not required for all diagrams, but it is required for c4c, mindmap and zenuml.
    const salt = Math.random().toString(36).substring(7);
    svg.value = `${svgCode} <span style="display: none">${salt}</span>`;
};
</script>
