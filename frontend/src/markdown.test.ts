import { describe, expect, it } from "vitest";

import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("renders supported headings lists tasks tables links and code", () => {
    const html = renderMarkdown(`# Heading

- [x] Done
- [ ] Remaining

| A | B |
| - | - |
| 1 | 2 |

[Docs](https://example.com)

\`\`\`ts
const safe = true;
\`\`\``);

    expect(html).toContain("<h1>Heading</h1>");
    expect(html).toContain('type="checkbox"');
    expect(html).toContain("<table>");
    expect(html).toContain("<code");
    expect(html).toContain("Docs");
  });

  it("blocks active HTML scripts handlers unsafe links and external resources", () => {
    const html = renderMarkdown(`<script>alert(1)</script>
<img src="https://tracking.invalid/pixel" onerror="alert(2)">
<a href="javascript:alert(3)" onclick="alert(4)">unsafe</a>
[Remote](https://example.com)
<div style="background:url(https://tracking.invalid)">text</div>`);

    expect(html).not.toMatch(/<script|<img|onerror|onclick|javascript:|style=/i);
    expect(html).not.toContain('href="https://example.com"');
    expect(html).toContain("Remote");
    expect(html).toContain("text");
  });

  it("removes injected forms and interactive controls", () => {
    const html = renderMarkdown(
      '<form action="https://evil.invalid"><input name="secret"><button type="submit">Send</button></form><details open><summary>Open</summary>payload</details>',
    );

    expect(html).not.toMatch(/<form|<button|<details|<summary|action=|name=/i);
    expect(html).toContain("Send");
    expect(html).toContain("payload");
  });
});
