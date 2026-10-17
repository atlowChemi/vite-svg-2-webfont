/** Save bytes or text as a file through a temporary object URL. */
export function downloadFile(fileName: string, bytes: BlobPart): void {
    const link = document.createElement('a');
    link.href = URL.createObjectURL(new Blob([bytes]));
    link.download = fileName;
    link.click();
    URL.revokeObjectURL(link.href);
}
