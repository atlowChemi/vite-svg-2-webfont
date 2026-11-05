import type { FC } from 'react';
import { ColophonText } from './Colophon.tsx';
import { ToolDrawers, toolDrawers } from './Drawers.tsx';
import { Features, foundryFeatures } from './Features.tsx';
import { Flyer } from './Flyer.tsx';
import { Footer } from './Footer.tsx';
import { Hero } from './Hero.tsx';
import { Outputs } from './Outputs.tsx';
import { Section, SectionHeading } from './Section.tsx';
import styles from './Landing.module.css';

interface Props {
    install: Record<string, string>;
}

/**
 * The landing page, a specimen sheet in five numbered sections. `install` holds index.md's rendered slots: a snippet per
 * drawer (`cli`, `node`, `wasm`, `vite`) and `outputs`. The page is static; `src/client/landing.ts` runs the pour, the
 * cooling and the drawers.
 */
export const Landing: FC<Props> = ({ install }) => (
    <div className={styles.landing}>
        <Flyer />
        <Hero />
        <Section num="02" name="Setting">
            <div className={styles.body}>
                <SectionHeading title="What are you casting into?" lede="One engine, four drawers in the type case." illustration="/engine-anywhere.svg" />
                <ToolDrawers drawers={toolDrawers} snippets={install} />
            </div>
        </Section>
        <Section num="03" name="Foundry">
            <div className={styles.body}>
                <SectionHeading title="Why cast with fonte" lede="A foundry built for icon sets." />
                <Features features={foundryFeatures} />
            </div>
        </Section>
        <Section num="04" name="Proof">
            <div className={styles.body}>
                <SectionHeading title="What comes out of the mold" lede="A folder of SVGs in, a font family and everything around it out." illustration="/flexible-output.svg" />
                <Outputs snippet={install.outputs ?? ''} />
            </div>
        </Section>
        <Section num="05" name="Colophon" id="colophon" className={styles.colophon}>
            <div className={styles.body}>
                <SectionHeading title="Colophon" lede="Where the name comes from." />
                <ColophonText />
            </div>
        </Section>
        <Footer />
    </div>
);
