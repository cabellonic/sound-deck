import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { usePlaybackActions } from '@/features/useSoundboard';
import * as ipc from '@/lib/ipc';
import type * as IpcModule from '@/lib/ipc';
import { useUiStore } from '@/stores/useUiStore';

vi.mock('@/lib/ipc', async () => {
  const actual = await vi.importActual<typeof IpcModule>('@/lib/ipc');
  return {
    ...actual,
    previewRemoteSound: vi.fn(),
    previewLocalSound: vi.fn().mockResolvedValue(undefined),
    stopPreview: vi.fn().mockResolvedValue(undefined),
  };
});

/** Promesa que se resuelve a mano, para congelar una descarga a mitad. */
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

const A = 'remote:freesound:1';
const B = 'remote:freesound:2';

describe('previsualizacion de un audio online', () => {
  beforeEach(() => {
    useUiStore.setState({ previewKey: null, previewLoadingKey: null });
  });

  it('no dice que suena hasta que termino de bajarse', async () => {
    const bajando = deferred();
    vi.mocked(ipc.previewRemoteSound).mockReturnValue(bajando.promise);

    const { result } = renderHook(() => usePlaybackActions());
    act(() => void result.current.previewRemote('freesound', '1'));

    // Mientras baja: cargando, nada sonando.
    expect(useUiStore.getState().previewLoadingKey).toBe(A);
    expect(useUiStore.getState().previewKey).toBeNull();

    await act(async () => {
      bajando.resolve();
      await bajando.promise;
    });

    expect(useUiStore.getState().previewKey).toBe(A);
    expect(useUiStore.getState().previewLoadingKey).toBeNull();
  });

  it('al pedir otra, la primera deja de sonar en el acto', async () => {
    vi.mocked(ipc.previewRemoteSound).mockResolvedValue(undefined);
    const { result } = renderHook(() => usePlaybackActions());

    await act(async () => {
      await result.current.previewRemote('freesound', '1');
    });
    expect(useUiStore.getState().previewKey).toBe(A);

    // Se pide la segunda: la primera se calla ya, sin esperar la descarga.
    const bajando = deferred();
    vi.mocked(ipc.previewRemoteSound).mockReturnValue(bajando.promise);
    act(() => void result.current.previewRemote('freesound', '2'));

    expect(useUiStore.getState().previewKey).toBeNull();
    expect(useUiStore.getState().previewLoadingKey).toBe(B);

    await act(async () => {
      bajando.resolve();
      await bajando.promise;
    });
    expect(useUiStore.getState().previewKey).toBe(B);
  });

  it('una descarga que llega tarde no se pone a sonar', async () => {
    const primera = deferred();
    const segunda = deferred();
    vi.mocked(ipc.previewRemoteSound).mockReturnValueOnce(primera.promise);

    const { result } = renderHook(() => usePlaybackActions());
    act(() => void result.current.previewRemote('freesound', '1'));

    vi.mocked(ipc.previewRemoteSound).mockReturnValueOnce(segunda.promise);
    act(() => void result.current.previewRemote('freesound', '2'));

    // La primera termina despues de la segunda: ya no es la vigente.
    await act(async () => {
      primera.resolve();
      await primera.promise;
    });

    expect(useUiStore.getState().previewKey).toBeNull();
    expect(useUiStore.getState().previewLoadingKey).toBe(B);

    await act(async () => {
      segunda.resolve();
      await segunda.promise;
    });
    expect(useUiStore.getState().previewKey).toBe(B);
  });

  it('se puede cancelar mientras baja', async () => {
    const bajando = deferred();
    vi.mocked(ipc.previewRemoteSound).mockReturnValue(bajando.promise);

    const { result } = renderHook(() => usePlaybackActions());
    act(() => void result.current.previewRemote('freesound', '1'));
    expect(useUiStore.getState().previewLoadingKey).toBe(A);

    await act(async () => {
      await result.current.stopPreview();
    });

    expect(useUiStore.getState().previewLoadingKey).toBeNull();
    await waitFor(() => expect(ipc.stopPreview).toHaveBeenCalled());

    // Y cuando la descarga termine, tampoco se pone a sonar.
    await act(async () => {
      bajando.resolve();
      await bajando.promise;
    });
    expect(useUiStore.getState().previewKey).toBeNull();
  });

  it('previsualizar uno local tambien corta la espera del online', async () => {
    const bajando = deferred();
    vi.mocked(ipc.previewRemoteSound).mockReturnValue(bajando.promise);

    const { result } = renderHook(() => usePlaybackActions());
    act(() => void result.current.previewRemote('freesound', '1'));

    await act(async () => {
      await result.current.previewLocal('sound-9');
    });

    expect(useUiStore.getState().previewKey).toBe('local:sound-9');
    expect(useUiStore.getState().previewLoadingKey).toBeNull();

    await act(async () => {
      bajando.resolve();
      await bajando.promise;
    });
    expect(useUiStore.getState().previewKey).toBe('local:sound-9');
  });
});
