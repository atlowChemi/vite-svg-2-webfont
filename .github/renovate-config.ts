import { env } from 'node:process';
import { fileURLToPath } from 'node:url';
import type { AllConfig } from 'renovate/dist/config/types';

const intentCreatorScriptUrl = new URL('../scripts/renovate-intent.mjs', import.meta.url);
const intentCommand = `node "${fileURLToPath(intentCreatorScriptUrl)}"`;

const config: AllConfig = {
    platform: 'github',
    repositories: [env.GITHUB_REPOSITORY!],
    onboarding: false,
    requireConfig: 'required',
    binarySource: 'global',
    // Adopt PRs and the dashboard created by the former Mend bot.
    ignorePrAuthor: true,
    exitCodeForErrors: true,
    allowedCommands: [`^${intentCommand.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`],
    postUpgradeTasks: {
        commands: [intentCommand],
        executionMode: 'branch',
        dataFileTemplate: '{{{branchName}}}\n{{#each upgrades}}{{{depName}}}: {{{newValue}}} {{{newDigest}}}\n{{/each}}',
        fileFilters: ['.changeset/renovate-*.md'],
    },
};

export default config;
