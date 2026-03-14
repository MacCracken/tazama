interface SliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  label: string;
  onChange: (value: number) => void;
}

export function Slider({ value, min, max, step = 0.01, label, onChange }: SliderProps) {
  return (
    <div>
      <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
        {label}: {value.toFixed(2)}
      </label>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-full"
      />
    </div>
  );
}
