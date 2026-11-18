import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { getBackend } from "../lib/api";

export function Markdown({ children }: { children: string }) {
  return (
    <div className="markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a({ href, children: content }) {
            return (
              <a
                href={href}
                onClick={(event) => {
                  event.preventDefault();
                  if (href) void getBackend().openExternal(href);
                }}
              >
                {content}
              </a>
            );
          }
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
