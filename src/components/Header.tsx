import { Logo } from './brand';
import { CommandPaletteButton, ModeIndicator, ProductModeSelector, ThemeToggle } from './controls';

export function Header() {
  return (
    <header className="flex h-16 items-center justify-between border-b border-slate-800/80 bg-slate-950/95 px-4 backdrop-blur">
      <div className="flex items-center gap-4">
        <Logo />
        <ModeIndicator />
      </div>

      <ProductModeSelector />

      <div className="flex items-center gap-2">
        <CommandPaletteButton />
        <ThemeToggle />
      </div>
    </header>
  );
}
