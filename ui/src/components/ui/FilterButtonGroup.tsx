interface FilterButtonGroupProps<T extends string> {
  options: readonly T[];
  selected: T;
  onChange: (value: T) => void;
  labelFn?: (value: T) => string;
}

export function FilterButtonGroup<T extends string>({
  options,
  selected,
  onChange,
  labelFn,
}: FilterButtonGroupProps<T>) {
  return (
    <div className="flex gap-1">
      {options.map((option) => (
        <button
          key={option}
          onClick={() => onChange(option)}
          className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
            selected === option
              ? "bg-gray-700 text-white"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          {labelFn ? labelFn(option) : option.charAt(0).toUpperCase() + option.slice(1)}
        </button>
      ))}
    </div>
  );
}
