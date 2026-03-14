interface NumberInputProps {
  value: number;
  label: string;
  min?: number;
  max?: number;
  step?: number;
  onChange: (value: number) => void;
}

export function NumberInput({
  value,
  label,
  min,
  max,
  step = 1,
  onChange,
}: NumberInputProps) {
  return (
    <div>
      <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
        {label}
      </label>
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
        className="w-full px-1.5 py-1 rounded text-xs"
        style={{
          background: "var(--bg-primary)",
          border: "1px solid var(--border-default)",
        }}
      />
    </div>
  );
}
