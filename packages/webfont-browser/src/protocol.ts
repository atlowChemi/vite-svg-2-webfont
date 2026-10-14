import type { CollectedOutput } from './types';

export type RequestPayload =
    | { method: 'generate'; options: string }
    | { method: 'regenerate'; membership: string; changes: string }
    | { method: 'font'; format: string }
    | { method: 'css' | 'html' | 'outputs' };
export type Request = { id: number } & RequestPayload;
export type Result = undefined | string | Uint8Array<ArrayBuffer> | CollectedOutput[];
export type Response = { id: number } & ({ ok: true; value: Result } | { ok: false; error: string; fatal: boolean });
