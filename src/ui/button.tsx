import { forwardRef } from "react";
import { cn } from "@/lib/utils";

type Variant = "primary" | "secondary" | "outline" | "ghost" | "danger";
type Size = "xs" | "sm" | "md" | "lg";

const VARIANTS: Record<Variant, string> = {
  primary: "border-transparent bg-brand text-brand-foreground hover:bg-brand/90",
  secondary: "border-transparent bg-secondary text-foreground hover:bg-secondary/70",
  outline: "border-border bg-transparent text-foreground hover:bg-muted",
  ghost: "border-transparent bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground",
  danger: "border-transparent bg-danger/10 text-danger hover:bg-danger/15",
};

const SIZES: Record<Size, string> = {
  xs: "h-6 gap-1 px-2 text-xs",
  sm: "h-7 gap-1.5 px-2.5 text-13",
  md: "h-8 gap-1.5 px-3.5 text-13",
  lg: "h-9 gap-2 px-4 text-sm",
};

const ICON_SIZES: Record<Size, string> = {
  xs: "size-6",
  sm: "size-7",
  md: "size-8",
  lg: "size-9",
};

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", className, type = "button", ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(
        "inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-ctl border font-medium",
        "transition-colors duration-150 active:translate-y-px disabled:pointer-events-none disabled:opacity-40",
        "[&_svg]:size-4 [&_svg]:shrink-0",
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...props}
    />
  );
});

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  variant?: Variant;
  size?: Size;
  active?: boolean;
}

/** Square button with an icon; `label` doubles as tooltip and accessible name. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { label, variant = "ghost", size = "md", active, className, type = "button", ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      title={label}
      aria-label={label}
      aria-pressed={active}
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-ctl border transition-colors duration-150",
        "active:translate-y-px disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-4",
        VARIANTS[variant],
        active && "bg-secondary text-foreground",
        ICON_SIZES[size],
        className,
      )}
      {...props}
    />
  );
});
