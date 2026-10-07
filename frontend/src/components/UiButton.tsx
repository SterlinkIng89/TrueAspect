import type { ButtonHTMLAttributes, ReactElement } from 'react'
import { cn } from '../lib/cn'

export interface UiButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly variant?: 'default' | 'primary' | 'ghost' | 'outline'
  readonly size?: 'sm' | 'md'
}

export function UiButton({
  children,
  className,
  variant = 'default',
  size = 'sm',
  type = 'button',
  ...props
}: UiButtonProps): ReactElement {
  return (
    <button
      type={type}
      className={cn(
        'inline-flex items-center justify-center gap-1.5 font-medium transition-colors cursor-pointer select-none disabled:opacity-40 disabled:pointer-events-none',
        size === 'sm' && 'px-2.5 py-1.5 rounded-md text-xs',
        size === 'md' && 'px-4 py-2 rounded-md text-xs',
        variant === 'default' &&
          'text-foreground bg-muted hover:bg-muted/80 active:bg-muted/60 border border-border',
        variant === 'primary' &&
          'text-primary-foreground bg-primary hover:bg-primary/90 active:bg-primary/80',
        variant === 'ghost' &&
          'text-muted-foreground hover:text-foreground hover:bg-muted/60 active:bg-muted/40',
        variant === 'outline' &&
          'text-foreground bg-card hover:bg-muted active:bg-muted/70 border border-border',
        className
      )}
      {...props}
    >
      {children}
    </button>
  )
}
