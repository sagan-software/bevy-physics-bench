/** Assign a persistent browser label so separate computers do not share one history series. */
export function machineName(storage: () => Pick<Storage, 'getItem' | 'setItem'>, identify: () => string): string {
  try {
    const existing = storage().getItem('bevy-physics-machine');
    if (existing && existing.length <= 120 && !/[\u0000-\u001f\u007f]/u.test(existing)) return existing;
  } catch { /* A blocked store still permits an explicitly exported run. */ }
  const generated = `Browser ${identify()}`;
  try { storage().setItem('bevy-physics-machine', generated); } catch { /* Keep the generated label for this page. */ }
  return generated;
}
