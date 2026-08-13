import DOMPurify from "dompurify";
import { marked } from "marked";

export function renderMarkdown(markdown: string): string {
  const rendered = marked.parse(markdown, {
    async: false,
    breaks: false,
    gfm: true,
  }) as string;
  const sanitized = DOMPurify.sanitize(rendered, {
    ALLOWED_TAGS: [
      "h1",
      "h2",
      "h3",
      "h4",
      "h5",
      "h6",
      "p",
      "strong",
      "em",
      "del",
      "blockquote",
      "ul",
      "ol",
      "li",
      "table",
      "thead",
      "tbody",
      "tr",
      "th",
      "td",
      "pre",
      "code",
      "a",
      "hr",
      "br",
      "input",
    ],
    ALLOWED_ATTR: ["href", "title", "type", "checked", "disabled", "class"],
  });

  const document = new DOMParser().parseFromString(sanitized, "text/html");
  for (const link of document.querySelectorAll("a")) {
    const href = link.getAttribute("href");
    if (href && !href.startsWith("#")) {
      link.dataset.inertHref = href;
      link.removeAttribute("href");
      link.setAttribute("aria-label", `${link.textContent ?? "Link"} (external link disabled)`);
    }
  }
  for (const checkbox of document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')) {
    checkbox.disabled = true;
  }
  return document.body.innerHTML;
}
