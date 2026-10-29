'use client';

import React from 'react';

/**
 * Minimal but faithful Reddit-style markdown renderer.
 *
 * Supports (matches Reddit's markdown spec from support.reddithelp.com):
 *   - **bold**, *italic*, ~~strikethrough~~, `code`
 *   - # H1, ## H2, ### H3
 *   - > blockquote
 *   - ```code blocks```
 *   - [links](url)
 *   - ![](url) images (we render as link)
 *   - - / * / 1. lists
 *   - tables (basic)
 *   - Horizontal rule ---
 *   - @mentions and r/subreddit and u/username as links
 *   - Escapes for \`*\_\[\] etc.
 *
 * Reddit's own renderer is more permissive; we match the common subset.
 */
export function Markdown({ content, className }: { content: string; className?: string }) {
  const blocks = parseBlocks(content);
  return (
    <div className={`prose prose-sm dark:prose-invert max-w-none ${className || ''}`}>
      {blocks.map((b, i) => renderBlock(b, i))}
    </div>
  );
}

type Block =
  | { type: 'code'; lang?: string; content: string }
  | { type: 'heading'; level: number; content: string }
  | { type: 'quote'; content: string }
  | { type: 'hr' }
  | { type: 'list'; ordered: boolean; items: string[] }
  | { type: 'table'; headers: string[]; rows: string[][] }
  | { type: 'paragraph'; content: string };

function parseBlocks(text: string): Block[] {
  const lines = text.split('\n');
  const blocks: Block[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    // Code block
    if (line.startsWith('```')) {
      const lang = line.slice(3).trim();
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) {
        code.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      blocks.push({ type: 'code', lang, content: code.join('\n') });
      continue;
    }
    // Heading
    const h = line.match(/^(#{1,6})\s+(.+)$/);
    if (h) {
      blocks.push({ type: 'heading', level: h[1].length, content: h[2] });
      i++;
      continue;
    }
    // Horizontal rule
    if (/^---+\s*$/.test(line) || /^\*\*\*+\s*$/.test(line)) {
      blocks.push({ type: 'hr' });
      i++;
      continue;
    }
    // Blockquote
    if (line.startsWith('>')) {
      const quote: string[] = [];
      while (i < lines.length && lines[i].startsWith('>')) {
        quote.push(lines[i].slice(1).trim());
        i++;
      }
      blocks.push({ type: 'quote', content: quote.join('\n') });
      continue;
    }
    // Table (markdown pipe table)
    if (line.includes('|') && i + 1 < lines.length && /^\s*\|?[\s-:|]+\|?\s*$/.test(lines[i + 1])) {
      const headers = splitTableRow(line);
      i += 2; // skip header + separator
      const rows: string[][] = [];
      while (i < lines.length && lines[i].includes('|')) {
        rows.push(splitTableRow(lines[i]));
        i++;
      }
      blocks.push({ type: 'table', headers, rows });
      continue;
    }
    // List (unordered)
    if (/^\s*[-*+]\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\s*[-*+]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*+]\s+/, ''));
        i++;
      }
      blocks.push({ type: 'list', ordered: false, items });
      continue;
    }
    // List (ordered)
    if (/^\s*\d+\.\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ''));
        i++;
      }
      blocks.push({ type: 'list', ordered: true, items });
      continue;
    }
    // Empty line — skip
    if (line.trim() === '') {
      i++;
      continue;
    }
    // Paragraph (collect consecutive non-empty, non-special lines)
    const para: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== '' &&
      !lines[i].startsWith('```') &&
      !lines[i].startsWith('#') &&
      !lines[i].startsWith('>') &&
      !/^\s*[-*+]\s+/.test(lines[i]) &&
      !/^\s*\d+\.\s+/.test(lines[i]) &&
      !/^---+\s*$/.test(lines[i])
    ) {
      para.push(lines[i]);
      i++;
    }
    blocks.push({ type: 'paragraph', content: para.join('\n') });
  }
  return blocks;
}

function splitTableRow(line: string): string[] {
  return line.trim().replace(/^\||\|$/g, '').split('|').map(c => c.trim());
}

function renderBlock(b: Block, key: number): React.ReactNode {
  switch (b.type) {
    case 'code':
      return (
        <pre key={key} className="bg-muted/40 border border-border rounded-md p-3 my-2 overflow-x-auto text-xs">
          <code className="font-mono">{b.content}</code>
        </pre>
      );
    case 'heading': {
      const Tag = (`h${Math.min(b.level, 6)}` as keyof JSX.IntrinsicElements);
      return <Tag key={key} className="font-bold mt-3 mb-1">{renderInline(b.content)}</Tag>;
    }
    case 'quote':
      return (
        <blockquote key={key} className="border-l-4 border-border pl-3 my-2 italic text-muted-foreground">
          {renderInline(b.content)}
        </blockquote>
      );
    case 'hr':
      return <hr key={key} className="my-3 border-border" />;
    case 'list':
      return b.ordered ? (
        <ol key={key} className="list-decimal pl-6 my-2 space-y-1">
          {b.items.map((it, j) => <li key={j}>{renderInline(it)}</li>)}
        </ol>
      ) : (
        <ul key={key} className="list-disc pl-6 my-2 space-y-1">
          {b.items.map((it, j) => <li key={j}>{renderInline(it)}</li>)}
        </ul>
      );
    case 'table':
      return (
        <div key={key} className="overflow-x-auto my-2">
          <table className="w-full border border-border text-sm">
            <thead className="bg-muted/40">
              <tr>{b.headers.map((h, j) => <th key={j} className="border border-border px-2 py-1 text-left font-semibold">{renderInline(h)}</th>)}</tr>
            </thead>
            <tbody>
              {b.rows.map((row, j) => (
                <tr key={j}>
                  {row.map((c, k) => <td key={k} className="border border-border px-2 py-1">{renderInline(c)}</td>)}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case 'paragraph':
      return <p key={key} className="my-1 leading-relaxed">{renderInline(b.content)}</p>;
  }
}

/** Render inline markdown: **bold**, *italic*, ~~strike~~, `code`, [link](url), r/sub, u/user, @mention */
function renderInline(text: string): React.ReactNode {
  // Tokenize using a single regex with alternation
  const pattern = /(\*\*([^*]+)\*\*)|(\*([^*]+)\*)|(~~([^~]+)~~)|(`([^`]+)`)|(\[([^\]]+)\]\(([^)]+)\))|(!\[([^\]]*)\]\(([^)]+)\))|(r\/([a-zA-Z0-9_]+))|(u\/([a-zA-Z0-9_-]+))|(@([a-zA-Z0-9_-]+))/g;
  const out: React.ReactNode[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  let key = 0;
  while ((m = pattern.exec(text)) !== null) {
    if (m.index > last) {
      out.push(text.slice(last, m.index));
    }
    if (m[1]) {
      out.push(<strong key={key++} className="font-semibold">{m[2]}</strong>);
    } else if (m[3]) {
      out.push(<em key={key++} className="italic">{m[4]}</em>);
    } else if (m[5]) {
      out.push(<s key={key++} className="line-through opacity-70">{m[6]}</s>);
    } else if (m[7]) {
      out.push(<code key={key++} className="bg-muted/40 px-1 py-0.5 rounded text-xs font-mono">{m[8]}</code>);
    } else if (m[9]) {
      out.push(
        <a key={key++} href={m[11]} target="_blank" rel="noopener noreferrer" className="text-blue-600 dark:text-blue-400 hover:underline">
          {m[10]}
        </a>
      );
    } else if (m[12]) {
      out.push(
        <a key={key++} href={m[14]} target="_blank" rel="noopener noreferrer" className="text-blue-600 dark:text-blue-400 hover:underline">
          📷 {m[13] || 'image'}
        </a>
      );
    } else if (m[15]) {
      out.push(
        <a key={key++} href={`#/r/${m[16]}`} className="text-blue-600 dark:text-blue-400 hover:underline font-medium">r/{m[16]}</a>
      );
    } else if (m[17]) {
      out.push(
        <a key={key++} href={`#/user/${m[18]}`} className="text-blue-600 dark:text-blue-400 hover:underline font-medium">u/{m[18]}</a>
      );
    } else if (m[19]) {
      out.push(
        <span key={key++} className="text-blue-600 dark:text-blue-400 font-medium">@{m[20]}</span>
      );
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}
