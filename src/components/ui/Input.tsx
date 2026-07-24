import { forwardRef, type InputHTMLAttributes } from 'react';

import { cn } from '@/lib/utils';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...props }, ref) {
    return (
      <input
        ref={ref}
        className={cn(
          'h-9 w-full rounded-md border border-border-subtle bg-surface-1 px-3 text-sm',
          'text-fg-default placeholder:text-fg-subtle',
          // El anillo lo pone el estilo global, y solo en modo teclado; con el
          // mouse alcanza con que se marque el borde.
          'focus:border-accent',
          'disabled:cursor-not-allowed disabled:opacity-60',
          className,
        )}
        {...props}
      />
    );
  },
);

interface FieldProps {
  label: string;
  hint?: string;
  htmlFor?: string;
  children: React.ReactNode;
  className?: string;
}

/** Campo con etiqueta accesible y ayuda opcional. */
export function Field({ label, hint, htmlFor, children, className }: FieldProps) {
  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <label htmlFor={htmlFor} className="text-sm font-medium text-fg-default">
        {label}
      </label>
      {children}
      {hint ? <p className="text-xs leading-relaxed text-fg-subtle">{hint}</p> : null}
    </div>
  );
}
