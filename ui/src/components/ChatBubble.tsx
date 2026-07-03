import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Props {
  text: string;
  streaming?: boolean;
}

export default function ChatBubble({ text, streaming }: Props) {
  return (
    <div className={`chat-bubble${streaming ? " streaming" : ""}`}>
      <div className="markdown">
        <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
      </div>
      {streaming && <span className="cursor">▌</span>}
    </div>
  );
}
