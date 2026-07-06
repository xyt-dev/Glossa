import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import Reasoning from "./Reasoning";

interface Props {
  text: string;
  streaming?: boolean;
  /** Thinking trace shown as a collapsible section at the top of the bubble. */
  reasoning?: string | null;
  reasoningStreaming?: boolean;
}

export default function ChatBubble({
  text,
  streaming,
  reasoning,
  reasoningStreaming,
}: Props) {
  return (
    <div className={`chat-bubble${streaming ? " streaming" : ""}`}>
      {reasoning ? <Reasoning text={reasoning} streaming={reasoningStreaming} /> : null}
      {text ? (
        <div className="markdown">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
        </div>
      ) : null}
      {streaming && <span className="cursor">▌</span>}
    </div>
  );
}
