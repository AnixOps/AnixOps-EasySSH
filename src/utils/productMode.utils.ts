import type { ProductMode } from '../productModes';

export function isLiteMode(mode: ProductMode): boolean {
  return mode === 'lite';
}

export function isStandardMode(mode: ProductMode): boolean {
  return mode === 'standard';
}

export function isProMode(mode: ProductMode): boolean {
  return mode === 'pro';
}

export function hasTerminalFeature(mode: ProductMode): boolean {
  return mode === 'standard' || mode === 'pro';
}

export function hasTeamFeature(mode: ProductMode): boolean {
  return mode === 'pro';
}

export function hasNativeTerminalOnly(mode: ProductMode): boolean {
  return mode === 'lite';
}
