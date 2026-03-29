export function Modal({
  children,
  title,
  onClose,
}: {
  children: React.ReactNode;
  title: string;
  onClose: () => void;
}) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div
        className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-md p-6 shadow-xl shadow-black/30"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-slate-50">{title}</h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-100 transition text-sm"
          >
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
