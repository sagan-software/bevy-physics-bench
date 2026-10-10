import { expect, it } from 'vitest';
import { machineName } from '../src/machine';

it('retains a browser identity and replaces invalid stored labels', () => {
  for (const initial of [null, '', 'bad\nlabel', 'x'.repeat(121)]) {
    let value = initial;
    const store = () => ({ getItem: () => value, setItem: (_key: string, next: string) => { value = next; } });
    expect(machineName(store, () => 'first')).toBe('Browser first');
    expect(machineName(store, () => 'second')).toBe('Browser first');
  }
});

it('permits a page-scoped identity when persistent browser storage is unavailable', () => {
  expect(machineName(() => { throw new Error('blocked'); }, () => 'ephemeral')).toBe('Browser ephemeral');
});
