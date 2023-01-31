export class NoIconsAvailableError extends Error {}

export class InvalidWriteFilesTypeError extends Error {
    constructor(types: string[]) {
        super(`WriteFiles option received invalid types: ${types.join(', ')}`);
    }
}
