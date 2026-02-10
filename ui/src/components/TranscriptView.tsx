import { useRef, useEffect } from "react";
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
            <div key={i} className="group flex items-start gap-2">
              <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
              <p className="text-sm leading-relaxed text-gray-200">{t.text}</p>
              <span className="ml-auto shrink-0 text-xs text-gray-600 opacity-0 group-hover:opacity-100">
                {(t.confidence * 100).toFixed(0)}%
              </span>
            </div>
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
