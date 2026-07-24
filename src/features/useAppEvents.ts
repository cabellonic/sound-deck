/**
 * Puente entre los eventos de Rust y el estado del frontend (§24).
 *
 * Cada evento invalida exactamente las consultas afectadas o actualiza el store
 * de interfaz. No hay polling en ningun lado.
 */
import { useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';

import { onAppEvent } from '@/lib/events';
import * as ipc from '@/lib/ipc';
import { useUiStore } from '@/stores/useUiStore';

import { queryKeys } from './queryKeys';

export function useAppEvents(): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    const store = useUiStore.getState();
    const disposers: Array<Promise<() => void>> = [];

    const refreshPlayback = () => {
      void ipc
        .getPlaybackStatus()
        .then((status) => {
          store.setPlayingSoundIds(status.playingSoundIds);
          if (!status.previewing) store.setPreviewKey(null);
        })
        .catch(() => {
          // Un fallo aca solo afecta a los indicadores visuales.
        });
    };

    disposers.push(onAppEvent('playback-started', refreshPlayback));
    disposers.push(onAppEvent('playback-stopped', refreshPlayback));

    disposers.push(
      onAppEvent('playback-error', (payload) => {
        store.pushToast('error', payload.message);
        refreshPlayback();
      }),
    );

    disposers.push(
      onAppEvent('library-changed', () => {
        void queryClient.invalidateQueries({ queryKey: ['sounds'] });
        void queryClient.invalidateQueries({ queryKey: queryKeys.facets });
        void queryClient.invalidateQueries({ queryKey: queryKeys.storage });
      }),
    );

    disposers.push(
      onAppEvent('page-changed', () => {
        void queryClient.invalidateQueries({ queryKey: queryKeys.pages });
        void queryClient.invalidateQueries({ queryKey: ['page'] });
      }),
    );

    disposers.push(
      onAppEvent('slot-changed', () => {
        void queryClient.invalidateQueries({ queryKey: ['page'] });
        void queryClient.invalidateQueries({ queryKey: queryKeys.pages });
        void queryClient.invalidateQueries({ queryKey: ['sounds'] });
      }),
    );

    disposers.push(
      onAppEvent('settings-changed', (settings) => {
        queryClient.setQueryData(queryKeys.settings, settings);
        void queryClient.invalidateQueries({ queryKey: queryKeys.appState });
      }),
    );

    disposers.push(
      onAppEvent('audio-device-changed', (payload) => {
        void queryClient.invalidateQueries({ queryKey: queryKeys.devices });
        if (payload.notice) store.pushToast('warning', payload.notice);
      }),
    );

    disposers.push(
      onAppEvent('audio-device-lost', (payload) => {
        void queryClient.invalidateQueries({ queryKey: queryKeys.devices });
        store.pushToast(
          'warning',
          payload.notice ?? `Se perdio el dispositivo "${payload.deviceName}".`,
        );
      }),
    );

    disposers.push(
      onAppEvent('download-progress', (payload) => {
        useUiStore.getState().setDownload(`${payload.providerId}:${payload.remoteId}`, {
          receivedBytes: payload.receivedBytes,
          totalBytes: payload.totalBytes,
          status: 'downloading',
        });
      }),
    );

    disposers.push(
      onAppEvent('download-completed', (payload) => {
        // Sin esto el resultado se queda girando para siempre aunque ya bajo.
        useUiStore.getState().setDownload(`${payload.providerId}:${payload.remoteId}`, null);
        void queryClient.invalidateQueries({ queryKey: ['sounds'] });
        void queryClient.invalidateQueries({ queryKey: queryKeys.facets });
      }),
    );

    disposers.push(
      onAppEvent('download-failed', (payload) => {
        useUiStore.getState().setDownload(`${payload.providerId}:${payload.remoteId}`, null);
        store.pushToast('error', payload.message);
      }),
    );

    disposers.push(
      onAppEvent('notice', (payload) => {
        store.pushToast(payload.level, payload.message);
      }),
    );

    disposers.push(
      onAppEvent('open-settings', () => {
        store.setSettingsOpen(true);
      }),
    );

    return () => {
      for (const disposer of disposers) {
        void disposer.then((unlisten) => unlisten()).catch(() => undefined);
      }
    };
  }, [queryClient]);
}
