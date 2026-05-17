// GOOD: uses the Rust NAPI binding instead of a banned JS package
import { sanitize } from '@example/rust-napi/sanitize-html';
import { markdownToHtml } from '@example/rust-napi/markdown-to-html';

export function renderDoc(markdown: string): string {
    const html = markdownToHtml(markdown);
    return sanitize(html);
}
