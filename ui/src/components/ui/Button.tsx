import type { ButtonHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "danger" | "ghost";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary:
    "bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white disabled:bg-blue-600/50",
  secondary:
    "bg-gray-700 hover:bg-gray-600 active:bg-gray-500 text-gray-100 disabled:bg-gray-700/50",
  danger:
    "bg-red-600 hover:bg-red-700 active:bg-red-800 text-white disabled:bg-red-600/50",
  ghost:
    "bg-transparent hover:bg-gray-700/50 active:bg-gray-700 text-gray-300 disabled:text-gray-600",
};

const sizeClasses: Record<Size, string> = {
  sm: "px-3 py-1.5 text-sm",
  md: "px-4 py-2 text-sm",
  lg: "px-6 py-3 text-base",
};

export function Button({
  variant = "primary",
  size = "md",
  className = "",
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      className={`inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500/50 disabled:cursor-not-allowed ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
