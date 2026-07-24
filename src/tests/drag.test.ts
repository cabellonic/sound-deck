import { describe, expect, it } from 'vitest';

import { isDragPayload } from '@/lib/drag';

describe('isDragPayload', () => {
  it('acepta cada tipo de payload bien formado', () => {
    expect(isDragPayload({ kind: 'local-sound', soundId: 'abc' })).toBe(true);
    expect(isDragPayload({ kind: 'remote-sound', providerId: 'freesound', remoteId: '42' })).toBe(
      true,
    );
    expect(isDragPayload({ kind: 'slot', pageId: 'p1', slotNumber: 5 })).toBe(true);
    expect(isDragPayload({ kind: 'page', pageId: 'p1' })).toBe(true);
  });

  it('rechaza tipos desconocidos o incompletos', () => {
    expect(isDragPayload(null)).toBe(false);
    expect(isDragPayload('texto')).toBe(false);
    expect(isDragPayload({ kind: 'otro' })).toBe(false);
    expect(isDragPayload({ kind: 'local-sound' })).toBe(false);
    expect(isDragPayload({ kind: 'local-sound', soundId: '' })).toBe(false);
    expect(isDragPayload({ kind: 'page', pageId: '' })).toBe(false);
  });

  it('rechaza un slot fuera del rango 1..9', () => {
    expect(isDragPayload({ kind: 'slot', pageId: 'p', slotNumber: 0 })).toBe(false);
    expect(isDragPayload({ kind: 'slot', pageId: 'p', slotNumber: 42 })).toBe(false);
    expect(isDragPayload({ kind: 'slot', pageId: 'p', slotNumber: 3.5 })).toBe(false);
  });
});
