// BAD: imports from banned CPU-intensive package
import sanitizeHtml from 'sanitize-html';

export function sanitizeEmailBody(html: string): string {
    return sanitizeHtml(html, {
        allowedTags: ['b', 'i', 'em', 'strong', 'a'],
        allowedAttributes: { a: ['href'] },
    });
}
