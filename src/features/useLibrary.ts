/**
 * Biblioteca local: busqueda, importacion y acciones sobre sonidos.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { open } from '@tauri-apps/plugin-dialog';

import * as ipc from '@/lib/ipc';
import { errorMessage } from '@/lib/ipc';
import { useUiStore } from '@/stores/useUiStore';
import type { ImportReport } from '@/types/domain';

import { queryKeys } from './queryKeys';
import { useDebouncedValue } from './useDebouncedValue';

/**
 * Busqueda local. Un retraso corto evita disparar una consulta por tecla sin
 * que se note lentitud (§26).
 */
export function useLocalSounds() {
  const searchText = useUiStore((state) => state.searchByTab.saved);
  const filter = useUiStore((state) => state.libraryFilter);
  const sort = useUiStore((state) => state.sortOrder);
  const debouncedText = useDebouncedValue(searchText, 120);

  return useQuery({
    queryKey: queryKeys.sounds(debouncedText, filter, sort),
    queryFn: () => ipc.searchLocalSounds({ text: debouncedText, filter, sort, limit: 500 }),
    // Mantener la lista anterior mientras llega la nueva evita el parpadeo.
    placeholderData: (previous) => previous,
  });
}

export function useLibraryFacets() {
  return useQuery({ queryKey: queryKeys.facets, queryFn: ipc.getLibraryFacets });
}

export function useLibraryStorage() {
  return useQuery({ queryKey: queryKeys.storage, queryFn: ipc.getLibraryStorage });
}

export function useAppFolders() {
  return useQuery({ queryKey: queryKeys.folders, queryFn: ipc.getAppFolders });
}

/** Resume el resultado de una importacion en un unico aviso claro (§33). */
function summarizeImport(report: ImportReport): { level: 'success' | 'warning'; message: string } {
  const parts: string[] = [];
  if (report.imported.length > 0) {
    parts.push(
      report.imported.length === 1
        ? `1 audio importado`
        : `${report.imported.length} audios importados`,
    );
  }
  if (report.duplicates.length > 0) {
    parts.push(`${report.duplicates.length} ya estaban en la biblioteca`);
  }
  if (report.failed.length > 0) {
    parts.push(`${report.failed.length} con error`);
  }

  if (parts.length === 0) {
    return { level: 'warning', message: 'No se importo ningun archivo.' };
  }

  return {
    level: report.failed.length > 0 ? 'warning' : 'success',
    message: `${parts.join(', ')}.`,
  };
}

export function useLibraryMutations() {
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);
  const onError = (error: unknown) => pushToast('error', errorMessage(error));

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ['sounds'] });
    void queryClient.invalidateQueries({ queryKey: queryKeys.facets });
    void queryClient.invalidateQueries({ queryKey: queryKeys.storage });
    void queryClient.invalidateQueries({ queryKey: ['page'] });
  };

  const importFiles = useMutation({
    mutationFn: (paths: string[]) => ipc.importSoundFiles(paths),
    onSuccess: (report) => {
      invalidate();
      const summary = summarizeImport(report);
      pushToast(summary.level, summary.message);

      // Los archivos rechazados se detallan uno por uno: el usuario necesita
      // saber cual fallo y por que (§29).
      for (const failure of report.failed.slice(0, 3)) {
        pushToast('error', `${failure.fileName}: ${failure.message}`);
      }
    },
    onError,
  });

  /** Abre el dialogo nativo de seleccion multiple. */
  const importWithDialog = async () => {
    try {
      const extensions = await ipc.supportedAudioExtensions();
      const selected = await open({
        multiple: true,
        title: 'Importar audios',
        filters: [{ name: 'Audio', extensions }],
      });

      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      if (paths.length > 0) await importFiles.mutateAsync(paths);
    } catch (error) {
      pushToast('error', errorMessage(error));
    }
  };

  const rename = useMutation({
    mutationFn: ({ soundId, name }: { soundId: string; name: string }) =>
      ipc.renameSound(soundId, name),
    onSuccess: invalidate,
    onError,
  });

  /** `volume: null` vuelve a linkear el audio al volumen general. */
  const setVolume = useMutation({
    mutationFn: ({ soundId, volume }: { soundId: string; volume: number | null }) =>
      ipc.updateSoundVolume(soundId, volume),
    onSuccess: invalidate,
    onError,
  });

  const setImage = useMutation({
    mutationFn: ({ soundId, path }: { soundId: string; path: string }) =>
      ipc.setSoundImage(soundId, path),
    onSuccess: invalidate,
    onError,
  });

  const clearImage = useMutation({
    mutationFn: (soundId: string) => ipc.clearSoundImage(soundId),
    onSuccess: () => {
      invalidate();
      pushToast('info', 'Imagen quitada.');
    },
    onError,
  });

  /**
   * Elige una imagen con el dialogo nativo y se la asigna al audio.
   *
   * El filtro de extensiones lo dicta el backend, que es quien despues valida
   * el contenido: la lista no es una promesa, solo una ayuda para el usuario.
   */
  const pickImageWithDialog = async (soundId: string) => {
    try {
      const extensions = await ipc.supportedImageExtensions();
      const selected = await open({
        multiple: false,
        title: 'Elegir imagen del audio',
        filters: [{ name: 'Imagen', extensions }],
      });

      if (typeof selected !== 'string') return;
      await setImage.mutateAsync({ soundId, path: selected });
    } catch (error) {
      pushToast('error', errorMessage(error));
    }
  };

  const setTags = useMutation({
    mutationFn: ({ soundId, tags }: { soundId: string; tags: string[] }) =>
      ipc.updateSoundTags(soundId, tags),
    onSuccess: invalidate,
    onError,
  });

  const remove = useMutation({
    mutationFn: (soundId: string) => ipc.deleteSound(soundId),
    onSuccess: () => {
      invalidate();
      pushToast('info', 'Audio eliminado de la biblioteca.');
    },
    onError,
  });

  const download = useMutation({
    mutationFn: ({ providerId, remoteId }: { providerId: string; remoteId: string }) =>
      ipc.downloadRemoteSound(providerId, remoteId),
    onSuccess: (sound) => {
      invalidate();
      pushToast('success', `"${sound.name}" guardado en la biblioteca.`);
    },
    onError,
  });

  return {
    importFiles,
    importWithDialog,
    rename,
    setVolume,
    setImage,
    clearImage,
    pickImageWithDialog,
    setTags,
    remove,
    download,
  };
}
