/**
 * Consultas y mutaciones de paginas, slots y reproduccion.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

import * as ipc from '@/lib/ipc';
import { errorMessage } from '@/lib/ipc';
import { useUiStore } from '@/stores/useUiStore';
import type { SlotNumber, SoundPage } from '@/types/domain';

import { queryKeys } from './queryKeys';

export function usePages() {
  return useQuery({
    queryKey: queryKeys.pages,
    queryFn: ipc.listPages,
  });
}

export function usePage(pageId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.page(pageId),
    queryFn: () => ipc.getPage(pageId),
  });
}

/** Invalida todo lo que depende de la botonera. */
function useInvalidateBoard() {
  const queryClient = useQueryClient();
  return useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: queryKeys.pages });
    void queryClient.invalidateQueries({ queryKey: ['page'] });
    void queryClient.invalidateQueries({ queryKey: ['sounds'] });
    void queryClient.invalidateQueries({ queryKey: queryKeys.facets });
  }, [queryClient]);
}

export function usePageMutations() {
  const invalidate = useInvalidateBoard();
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);

  const onError = (error: unknown) => pushToast('error', errorMessage(error));

  const setPage = (page: SoundPage) => {
    queryClient.setQueryData(queryKeys.page(page.id), page);
    queryClient.setQueryData(queryKeys.page(undefined), page);
  };

  const create = useMutation({
    mutationFn: (name: string) => ipc.createPage(name),
    onSuccess: (page) => {
      setPage(page);
      invalidate();
      pushToast('success', `Pagina "${page.name}" creada.`);
    },
    onError,
  });

  const rename = useMutation({
    mutationFn: ({ pageId, name }: { pageId: string; name: string }) =>
      ipc.renamePage(pageId, name),
    onSuccess: (page) => {
      setPage(page);
      invalidate();
    },
    onError,
  });

  const remove = useMutation({
    mutationFn: (pageId: string) => ipc.deletePage(pageId),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: ['page'] });
      invalidate();
      pushToast('info', 'Pagina eliminada. Los audios siguen en la biblioteca.');
    },
    onError,
  });

  const reorder = useMutation({
    mutationFn: (pageIds: string[]) => ipc.reorderPages(pageIds),
    onSuccess: invalidate,
    onError,
  });

  const duplicate = useMutation({
    mutationFn: (pageId: string) => ipc.duplicatePage(pageId),
    onSuccess: (page) => {
      setPage(page);
      invalidate();
      pushToast('success', `Se creo "${page.name}".`);
    },
    onError,
  });

  const activate = useMutation({
    mutationFn: (pageId: string) => ipc.setActivePage(pageId),
    onSuccess: (page) => {
      setPage(page);
      void queryClient.invalidateQueries({ queryKey: ['page'] });
    },
    onError,
  });

  return { create, rename, remove, reorder, duplicate, activate };
}

export function useSlotMutations() {
  const invalidate = useInvalidateBoard();
  const pushToast = useUiStore((state) => state.pushToast);
  const onError = (error: unknown) => pushToast('error', errorMessage(error));

  const assign = useMutation({
    mutationFn: ({
      pageId,
      slotNumber,
      soundId,
    }: {
      pageId: string;
      slotNumber: SlotNumber;
      soundId: string;
    }) => ipc.assignSoundToSlot(pageId, slotNumber, soundId),
    onSuccess: (slot) => {
      invalidate();
      pushToast('success', `Asignado al boton ${slot.slotNumber}.`);
    },
    onError,
  });

  const clear = useMutation({
    mutationFn: ({ pageId, slotNumber }: { pageId: string; slotNumber: SlotNumber }) =>
      ipc.clearSlot(pageId, slotNumber),
    onSuccess: invalidate,
    onError,
  });

  const swap = useMutation({
    mutationFn: (input: {
      fromPageId: string;
      fromSlot: SlotNumber;
      toPageId: string;
      toSlot: SlotNumber;
    }) => ipc.swapSlots(input.fromPageId, input.fromSlot, input.toPageId, input.toSlot),
    onSuccess: invalidate,
    onError,
  });

  const setLabel = useMutation({
    mutationFn: ({
      pageId,
      slotNumber,
      label,
    }: {
      pageId: string;
      slotNumber: SlotNumber;
      label: string | null;
    }) => ipc.setSlotLabel(pageId, slotNumber, label),
    onSuccess: invalidate,
    onError,
  });

  const setVolume = useMutation({
    mutationFn: ({
      pageId,
      slotNumber,
      volume,
    }: {
      pageId: string;
      slotNumber: SlotNumber;
      volume: number | null;
    }) => ipc.setSlotVolume(pageId, slotNumber, volume),
    onSuccess: invalidate,
    onError,
  });

  const downloadAndAssign = useMutation({
    mutationFn: ({
      providerId,
      remoteId,
      pageId,
      slotNumber,
    }: {
      providerId: string;
      remoteId: string;
      pageId: string;
      slotNumber: SlotNumber;
    }) => ipc.downloadAndAssignRemoteSound(providerId, remoteId, pageId, slotNumber),
    onSuccess: (slot) => {
      invalidate();
      pushToast('success', `Descargado y asignado al boton ${slot.slotNumber}.`);
    },
    onError,
  });

  return { assign, clear, swap, setLabel, setVolume, downloadAndAssign };
}

/** Acciones de reproduccion con manejo de error unificado. */
export function usePlaybackActions() {
  const pushToast = useUiStore((state) => state.pushToast);
  const setPreviewKey = useUiStore((state) => state.setPreviewKey);
  const setPreviewLoadingKey = useUiStore((state) => state.setPreviewLoadingKey);

  const report = (error: unknown) => pushToast('error', errorMessage(error));

  /** Deja de sonar y de esperar cualquier previsualizacion anterior. */
  const clearPreview = () => {
    setPreviewKey(null);
    setPreviewLoadingKey(null);
  };

  return {
    playSlot: (pageId: string, slotNumber: SlotNumber) =>
      ipc.playSlot(pageId, slotNumber).catch(report),
    playSound: (soundId: string) => ipc.playSound(soundId).catch(report),
    previewLocal: (soundId: string) => {
      clearPreview();
      setPreviewKey(`local:${soundId}`);
      return ipc.previewLocalSound(soundId).catch((error) => {
        setPreviewKey(null);
        report(error);
      });
    },
    previewRemote: (providerId: string, remoteId: string) => {
      const key = `remote:${providerId}:${remoteId}`;
      // Suena recien cuando termina de bajarse: hasta entonces no hay nada
      // sonando, y decir lo contrario haria parecer que la aplicacion se colgo.
      clearPreview();
      setPreviewLoadingKey(key);

      return ipc
        .previewRemoteSound(providerId, remoteId)
        .then(() => {
          // Si mientras se bajaba el usuario pidio otra cosa, esta ya no suena:
          // el backend la descarto y aca no hay que marcarla como sonando.
          if (useUiStore.getState().previewLoadingKey !== key) return;
          setPreviewLoadingKey(null);
          setPreviewKey(key);
        })
        .catch((error) => {
          if (useUiStore.getState().previewLoadingKey === key) setPreviewLoadingKey(null);
          report(error);
        });
    },
    stopPreview: () => {
      clearPreview();
      return ipc.stopPreview().catch(report);
    },
    stopAll: () => {
      clearPreview();
      return ipc.stopAll().catch(report);
    },
  };
}
