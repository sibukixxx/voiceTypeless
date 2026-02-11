import { useRef, useEffect, useState } from "react";
import { useSessionStore } from "../store/sessionStore";

export function TranscriptView() {
  const partialTranscript = useSessionStore((s) => s.partialTranscript);
  const finalTranscripts = useSessionStore((s) => s.finalTranscripts);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom on new content
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [partialTranscript, finalTranscripts]);

  const isEmpty = finalTranscripts.length === 0 && !partialTranscript;

  return (
    <div
      ref={scrollRef}
      className="flex-1 overflow-y-auto rounded-xl border border-gray-800 bg-gray-950/50 p-4"
    >
      {isEmpty ? (
        <div className="flex h-full items-center justify-center">
          <p className="text-sm text-gray-600">
            Start recording to see transcripts here
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {/* Confirmed transcripts */}
          {finalTranscripts.map((t, i) => (
            <TranscriptItem key={t.segmentId ?? i} transcript={t} />
          ))}
          {/* Partial transcript (live) */}
          {partialTranscript && (
            <div className="flex items-start gap-2">
              <span className="mt-1 h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-blue-400" />
              <p className="text-sm leading-relaxed text-gray-400 italic">
                {partialTranscript}
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function TranscriptItem({
  transcript,
}: {
  transcript: {
    text: string;
    confidence: number;
    rawText?: string;
    rewrittenText?: string;
    isRewriting?: boolean;
  };
}) {
  const [showRaw, setShowRaw] = useState(false);
  const hasRewrite = !!transcript.rewrittenText;
  const isRewriting = transcript.isRewriting;

  // Determine indicator color
  let indicatorClass = "bg-green-500"; // default: confirmed
  if (isRewriting) {
    indicatorClass = "bg-yellow-400 animate-pulse";
  } else if (hasRewrite) {
    indicatorClass = "bg-purple-400";
  }

  return (
    <div className="group flex items-start gap-2">
      <span className={`mt-1 h-1.5 w-1.5 shrink-0 rounded-full ${indicatorClass}`} />
      <div className="min-w-0 flex-1">
        {isRewriting ? (
          <p className="text-sm leading-relaxed text-yellow-300/80 italic">
            Rewriting...
          </p>
        ) : (
          <p className="text-sm leading-relaxed text-gray-200">
            {transcript.text}
          </p>
        )}

        {/* Show raw text toggle for rewritten items */}
        {hasRewrite && transcript.rawText && !isRewriting && (
          <details
            className="mt-1"
            open={showRaw}
            onToggle={(e) =>
              setShowRaw((e.target as HTMLDetailsElement).open)
            }
          >
            <summary className="cursor-pointer text-xs text-gray-500 hover:text-gray-400">
              Original text
            </summary>
            <p className="mt-1 text-xs leading-relaxed text-gray-500">
              {transcript.rawText}
            </p>
          </details>
        )}
      </div>
      <span className="ml-auto shrink-0 text-xs text-gray-600 opacity-0 group-hover:opacity-100">
        {(transcript.confidence * 100).toFixed(0)}%
      </span>
    </div>
  );
}
