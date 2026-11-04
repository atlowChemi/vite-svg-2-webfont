// The Playground: rendered in the browser into the element the static page leaves for it, replacing its fallback text.
import '../components/playground/playground.css';
import { createRoot } from 'react-dom/client';
import { Playground } from '../components/playground/Playground.tsx';
import { PLAYGROUND_ROOT_ID } from '../components/playground/PlaygroundPage.tsx';

const root = document.getElementById(PLAYGROUND_ROOT_ID);
if (root) createRoot(root).render(<Playground />);
