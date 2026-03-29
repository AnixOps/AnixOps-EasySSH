export function RadioButton({
  label,
  description,
  checked,
  onChange,
  value,
  name,
  disabled = false,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange?: (checked: boolean) => void;
  value?: string;
  name?: string;
  disabled?: boolean;
}) {
  return (
    <label className={`flex items-center gap-3 p-3 rounded-xl cursor-pointer transition border border-slate-700 ${
      disabled
        ? 'opacity-50 cursor-not-allowed bg-slate-800/50'
        : 'hover:bg-slate-700 bg-slate-800'
    }`}>
      <input
        type="radio"
        name={name}
        checked={checked}
        onChange={(e) => !disabled && onChange?.(e.target.checked)}
        value={value}
        disabled={disabled}
        aria-label={label}
        className="w-4 h-4 text-cyan-500 bg-slate-900 border-slate-700 focus:ring-cyan-500/50 focus:ring-offset-0"
      />
      <div>
        <span className="text-slate-100 font-medium">{label}</span>
        {description && <p className="text-xs text-slate-500">{description}</p>}
      </div>
    </label>
  );
}
