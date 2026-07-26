/**
 * Catalogo en espanol. Es la fuente de la verdad: define las claves que
 * TypeScript exige, asi que agregar un texto empieza siempre por aca.
 *
 * Convenciones:
 * - Las claves se agrupan por pantalla con puntos (`library.empty.title`).
 * - Los parametros van entre llaves (`{name}`) y se completan al traducir.
 * - Los plurales viven en dos claves con sufijo `.one` y `.many`.
 */
export const es = {
  // --- Comunes --------------------------------------------------------------
  'common.cancel': 'Cancelar',
  'common.save': 'Guardar',
  'common.confirm': 'Confirmar',
  'common.delete': 'Borrar',
  'common.remove': 'Eliminar',
  'common.close': 'Cerrar',
  'common.open': 'Abrir',
  'common.import': 'Importar',
  'common.export': 'Exportar',
  'common.retry': 'Reintentar',
  'common.loading': 'Un momento...',
  'common.unknown': 'Desconocida',
  'common.none': '—',
  'common.unexpectedError': 'Ocurrio un error inesperado.',

  // --- Cabecera de la ventana principal --------------------------------------
  'app.starting': 'Iniciando Sound Deck...',
  'app.stop': 'Detener',
  'app.stopAll': 'Detener todos los sonidos',
  'app.nothingPlaying': 'No hay nada sonando en este momento',
  'app.overlay': 'Overlay',
  'app.openOverlay': 'Abrir overlay ({accelerator})',
  'app.settings': 'Configuracion',
  'app.openSettings': 'Abrir configuracion',
  'app.noActivePage': 'No hay ninguna pagina activa.',

  // --- Botonera --------------------------------------------------------------
  'soundboard.title': 'Botonera',
  'soundboard.label': 'Botonera de la pagina {page}',
  'soundboard.newPage': 'Nueva pagina (maximo {max})',
  'soundboard.createPage': 'Crear pagina',
  'soundboard.pageActions': 'Acciones de la pagina',
  'soundboard.renamePage': 'Renombrar pagina',
  'soundboard.duplicatePage': 'Duplicar pagina',
  'soundboard.deletePage': 'Borrar pagina',
  'soundboard.pages': 'Paginas de la botonera',
  'soundboard.previousPage': 'Pagina anterior',
  'soundboard.nextPage': 'Pagina siguiente',
  'soundboard.pageSlots': '{page}, {assigned} de {total} botones asignados',

  'slot.label': 'Boton {number}. {name}. {state}.',
  'slot.empty': 'Vacio',
  'slot.unassigned': 'Sin asignar',
  'slot.downloading': 'Descargando',
  'slot.unavailable': 'Archivo no disponible',
  'slot.missingFile': 'Archivo faltante',
  'slot.playing': 'Reproduciendo',
  'slot.assigned': 'Asignado',
  'slot.downloadProgress': 'Progreso de descarga',
  'slot.changeImage': 'Cambiar imagen del audio',
  'slot.setImage': 'Poner imagen al audio',

  // --- Biblioteca ------------------------------------------------------------
  'library.title': 'Biblioteca de audios',
  'library.searchSaved': 'Buscar en tus audios...',
  'library.searchOnline': 'Buscar audios en Internet...',
  'library.searchSavedLabel': 'Buscar audios guardados',
  'library.searchOnlineLabel': 'Buscar audios online',
  'library.clearSearch': 'Limpiar busqueda',
  'library.tabSaved': 'Guardados',
  'library.tabInternet': 'Internet',
  'library.loading': 'Cargando biblioteca',
  'library.emptyTitle': 'Tu biblioteca esta vacia',
  'library.emptyDescription':
    'Importa archivos desde tu computadora o busca audios en la pestana Internet.',
  'library.noResultsTitle': 'Sin resultados',
  'library.noResultsDescription': 'Proba con otras palabras o cambia el filtro activo.',
  'library.importSounds': 'Importar audios',
  'library.noProvidersTitle': 'Sin proveedores configurados',
  'library.noProvidersDescription':
    'Activa un proveedor online para buscar audios en Internet. Algunos necesitan una API key propia.',
  'library.configureProviders': 'Configurar proveedores',
  'library.searchOnlineTitle': 'Busca audios en Internet',
  'library.searchOnlineDescription':
    'Escribi al menos dos caracteres. Los resultados se previsualizan sin descargarse; solo se guardan cuando vos lo pedis.',
  'library.searching': 'Buscando...',
  'library.searchingDescription': 'Consultando proveedores.',
  'library.noRemoteResults': 'Ningun proveedor devolvio audios para "{query}".',
  'library.loadMore': 'Cargar mas resultados',

  'sound.preview': 'Previsualizar {name}',
  'sound.stopPreview': 'Detener {name}',
  'sound.missingFile': 'El archivo ya no esta en la carpeta de la aplicacion',
  'sound.missingFileLabel': 'Archivo no disponible',
  'sound.customVolume': 'Volumen propio: {percent}%, sin seguir el general',
  'sound.imported': 'Importado',
  'sound.assignTo': 'Asignar a un boton',
  'sound.assignToLabel': 'Asignar {name} a un boton',
  'sound.actions': 'Acciones para {name}',
  'sound.assignMenu': 'Asignar a...',
  'sound.rename': 'Renombrar',
  'sound.editVolume': 'Ajustar volumen',
  'sound.changeImage': 'Cambiar imagen',
  'sound.setImage': 'Poner imagen',
  'sound.clearImage': 'Quitar imagen',
  'sound.openFolder': 'Abrir carpeta',
  'sound.viewSource': 'Ver origen',
  'sound.deleteFromLibrary': 'Eliminar de la biblioteca',
  'sound.assignedTo': 'En {page} · boton {slot}',
  'sound.assignedCount.one': 'En {count} boton',
  'sound.assignedCount.many': 'En {count} botones',

  'remote.noPreview': '{name} no ofrece previsualizacion',
  'remote.saved': 'Guardado',
  'remote.openSource': 'Abrir pagina de origen',
  'remote.openSourceLabel': 'Abrir la pagina de origen de {name}',
  'remote.assign': 'Descargar y asignar a un boton',
  'remote.download': 'Descargar y guardar en la biblioteca',
  'remote.downloadLabel': 'Descargar {name}',
  'remote.downloading': 'Descargando {name}',

  // --- Filtros ---------------------------------------------------------------
  'filter.all': 'Todos',
  'filter.recent': 'Recientes',
  'filter.mostPlayed': 'Mas usados',
  'filter.unassigned': 'Sin asignar',
  'filter.more': 'Mas',
  'filter.moreLabel': 'Mas filtros',
  'filter.categories': 'Categorias',
  'filter.providers': 'Proveedores',
  'filter.sortBy': 'Ordenar por',
  'sort.relevance': 'Relevancia',
  'sort.recent': 'Reciente',
  'sort.mostPlayed': 'Mas usado',
  'sort.name': 'Nombre',
  'filter.sortByValue': 'Ordenar por {value}',

  // --- Categorias (espejo de NormalizedCategory en Rust) ---------------------
  'category.memes': 'Memes',
  'category.reactions': 'Reacciones',
  'category.games': 'Juegos',
  'category.anime': 'Anime',
  'category.movies_tv': 'Cine y TV',
  'category.music': 'Musica',
  'category.sound_effects': 'Efectos',
  'category.voices': 'Voces',
  'category.sports': 'Deportes',
  'category.other': 'Otros',
  'category.uncategorized': 'Sin categoria',

  // --- Dialogos --------------------------------------------------------------
  'dialog.assignTitle': 'Asignar a un boton',
  'dialog.assignDescription': 'Elegi la pagina y el boton para "{name}".',
  'dialog.page': 'Pagina',
  'dialog.pageTarget': 'Pagina de destino',
  'dialog.choosePage': 'Elegi una pagina',
  'dialog.button': 'Boton',
  'dialog.buttonTarget': 'Boton de destino',
  'dialog.slotTaken': 'Ocupado',
  'dialog.slotFree': 'Libre',
  'dialog.replaceHint': 'Elegir un boton ocupado reemplaza su asignacion actual.',

  'dialog.newPageTitle': 'Nueva pagina',
  'dialog.newPageDescription':
    'Las paginas son colecciones libres: pueden mezclar audios de cualquier origen.',
  'dialog.name': 'Nombre',
  'dialog.create': 'Crear',
  'dialog.renamePageTitle': 'Renombrar pagina',
  'dialog.deletePageTitle': 'Borrar pagina',
  'dialog.deletePageDescription': 'Se va a borrar "{name}". Los audios siguen en la biblioteca.',
  'dialog.deletePageAssigned.one': 'Esta pagina tiene {count} boton asignado.',
  'dialog.deletePageAssigned.many': 'Esta pagina tiene {count} botones asignados.',
  'dialog.renameSoundTitle': 'Renombrar audio',
  'dialog.soundVolumeTitle': 'Volumen del audio',
  'dialog.soundVolumeDescription': 'Vale para este audio en toda la aplicacion.',
  'dialog.followMaster': 'Seguir el volumen general',
  'dialog.followMasterHint':
    'Al desactivarlo, este audio suena siempre al valor que elijas, aunque muevas el general (ahora en {percent}%).',
  'dialog.deleteSoundTitle': 'Eliminar audio',
  'dialog.deleteSoundDescription': 'Se va a borrar "{name}" de la biblioteca y del disco.',
  'dialog.deleteSoundUsage': 'Se va a quitar de estos botones:',
  'dialog.usageItem': '{page} — boton {slot}',
  'dialog.slotLabelTitle': 'Nombre visible del boton',
  'dialog.slotLabelDescription':
    'Solo cambia lo que se muestra en este boton, no el nombre del audio.',
  'dialog.slotLabelField': 'Etiqueta',
  'dialog.slotVolumeTitle': 'Volumen del boton',
  'dialog.slotVolumeDescription': 'Solo cambia como suena en este boton.',
  'dialog.followSound': 'Usar el volumen del audio',
  'dialog.followSoundHint':
    'Al desactivarlo, este boton queda fijo en el valor que elijas, aunque el audio suene distinto en otros botones.',
  'dialog.downloadForImageTitle': 'Descargar el audio',
  'dialog.downloadForImageDescription':
    '"{name}" todavia no esta en tu biblioteca. Para ponerle una imagen hay que descargarlo primero.',
  'dialog.downloadForImageConfirm': 'Descargar y poner imagen',

  'details.title': 'Metadata del audio',
  'details.name': 'Nombre',
  'details.originalName': 'Nombre original',
  'details.duration': 'Duracion',
  'details.format': 'Formato',
  'details.size': 'Tamano',
  'details.category': 'Categoria',
  'details.tags': 'Etiquetas',
  'details.origin': 'Origen',
  'details.importedLocally': 'Importado localmente',
  'details.license': 'Licencia',
  'details.attribution': 'Atribucion',
  'details.volume': 'Volumen',
  'details.followsMaster': 'Sigue el volumen general',
  'details.image': 'Imagen',
  'details.imageAssigned': 'Asignada',
  'details.imageMissing': 'Sin imagen',
  'details.playCount': 'Reproducciones',
  'details.lastPlayed': 'Ultima vez',
  'details.added': 'Agregado',
  'details.file': 'Archivo',
  'details.fileAvailable': 'Disponible',
  'details.fileMissing': 'No encontrado',

  'date.never': 'Nunca',
  'date.now': 'Hace instantes',
  'date.minutes': 'Hace {value} min',
  'date.hours': 'Hace {value} h',
  'date.days': 'Hace {value} d',

  // --- Overlay ---------------------------------------------------------------
  'overlay.title': 'Overlay de Sound Deck',
  'overlay.noPage': 'Sin pagina',
  'overlay.close': 'Cerrar overlay',
  'overlay.buttons': 'Botones de la pagina',
  'overlay.placementHint': 'Arrastra desde cualquier parte para moverlo.',
  'overlay.placementResizeHint': 'Tira de la esquina: el alto se ajusta solo.',
  'overlay.placementResize': 'Cambiar el tamano del overlay',
  'overlay.placementSave': 'Guardar',

  // --- Onboarding ------------------------------------------------------------
  'onboarding.title': 'Bienvenido a Sound Deck',
  'onboarding.description':
    'Una soundboard local: tus audios viven en tu computadora y funcionan sin conexion.',
  'onboarding.importTitle': 'Importa tus audios',
  'onboarding.importDescription':
    'MP3, WAV, OGG o FLAC. Se copian a una carpeta propia para que nunca se rompan.',
  'onboarding.outputTitle': 'Elegi donde suena',
  'onboarding.outputDescription':
    'Para enviarlo a Discord necesitas un dispositivo virtual (VB-Cable o similar) ya instalado.',
  'onboarding.boardTitle': 'Arma tu botonera',
  'onboarding.boardDescription':
    'Arrastra audios a los nueve botones. {accelerator} abre el overlay sobre cualquier juego y las teclas 1 a 9 los disparan.',
  'onboarding.providersTitle': 'Busca audios online (opcional)',
  'onboarding.providersDescription':
    'Activa un proveedor y carga su API key para buscar en Internet desde la pestana correspondiente.',
  'onboarding.testAudio': 'Probar el audio',
  'onboarding.configureProvider': 'Configurar proveedor',
  'onboarding.start': 'Empezar',
  'onboarding.testedDevice': 'Si escuchaste un tono corto, el audio esta funcionando.',

  // --- Avisos ----------------------------------------------------------------
  'dialog.importSounds': 'Importar audios',
  'dialog.pickImage': 'Elegir imagen del audio',
  'dialog.imageFilter': 'Imagen',
  'toast.closeNotice': 'Cerrar aviso',

  // --- Nombres de tecla que cambian con el idioma ---------------------------
  'key.space': 'Espacio',
  'key.pageUp': 'Re Pag',
  'key.pageDown': 'Av Pag',
  'key.home': 'Inicio',
  'key.end': 'Fin',

  // --- Ajustes: pestanas -----------------------------------------------------
  'settings.title': 'Configuracion',
  'settings.tab.general': 'General',
  'settings.tab.audio': 'Audio',
  'settings.tab.shortcuts': 'Atajos',
  'settings.tab.library': 'Biblioteca',
  'settings.tab.providers': 'Proveedores',
  'settings.tab.advanced': 'Avanzado',
  'settings.tab.credits': 'Creditos',
  'settings.resolvingPath': 'Resolviendo...',

  // --- Ajustes: general ------------------------------------------------------
  'settings.general.autostart': 'Iniciar con el sistema',
  'settings.general.autostartHint':
    'Sound Deck arranca minimizado en la bandeja al iniciar sesion.',
  'settings.general.minimizeToTray': 'Minimizar a bandeja',
  'settings.general.minimizeToTrayHint':
    'Minimizar oculta la ventana en lugar de la barra de tareas.',
  'settings.general.closeToTray': 'Cerrar a bandeja',
  'settings.general.closeToTrayHint': 'La X oculta la ventana y la aplicacion sigue corriendo.',
  'settings.general.notifications': 'Mostrar notificaciones',
  'settings.general.overlayActiveMonitor': 'Abrir overlay en el monitor activo',
  'settings.general.overlayActiveMonitorIgnored':
    'Sin efecto: el overlay tiene una posicion elegida a mano.',
  'settings.general.overlayPosition': 'Posicion del overlay',
  'settings.general.overlayPositionFixed': 'Fija en {x}, {y}.',
  'settings.general.overlayPositionAuto':
    'Se centra solo. Podes elegir un lugar fijo y arrastrarlo hasta ahi.',
  'settings.general.overlayCenter': 'Centrar solo',
  'settings.general.overlayPick': 'Mover y redimensionar',
  'settings.general.overlaySize': 'Tamano del overlay',
  'settings.general.overlaySizeHint':
    'Mide {width} px de ancho; el alto lo ajusta el contenido. Tambien podes tirar de la esquina mientras lo moves.',
  'settings.general.overlaySizeSmall': 'Chico',
  'settings.general.overlaySizeMedium': 'Mediano',
  'settings.general.overlaySizeLarge': 'Grande',
  'settings.general.overlaySizeCustom': 'Personalizado',
  'settings.general.closeOverlayAfterPlay': 'Cerrar overlay despues de reproducir',
  'settings.general.closeOverlayAfterPlayHint':
    'Recomendado: vuelve el foco al juego o programa anterior.',
  'settings.general.closeOverlayOnBlur': 'Cerrar overlay al perder el foco',
  'settings.general.rememberLastPage': 'Recordar la ultima pagina',
  'settings.general.theme': 'Tema',
  'settings.general.themeSystem': 'Sistema',
  'settings.general.themeDark': 'Oscuro',
  'settings.general.themeLight': 'Claro',
  'settings.general.language': 'Idioma',
  'settings.general.languageHintMany': 'Cambia el idioma de toda la interfaz al instante.',
  'settings.general.languageHintOne':
    'Por ahora solo espanol. Los textos ya viven en un catalogo aparte: agregar un idioma no toca la interfaz.',

  // --- Ajustes: audio --------------------------------------------------------
  'settings.audio.device': 'Dispositivo de salida',
  'settings.audio.deviceHint': 'Se recuerda entre reinicios y se reconecta al arrancar.',
  'settings.audio.deviceDefault': 'Predeterminado del sistema',
  'settings.audio.deviceIsDefault': ' (predeterminado)',
  'settings.audio.refreshDevices': 'Actualizar dispositivos',
  'settings.audio.test': 'Probar',
  'settings.audio.playingOn': 'Sonando en "{device}".',
  'settings.audio.output': 'Salida: {device}',
  'settings.audio.masterVolume': 'Volumen general — {percent}%',
  'settings.audio.masterVolumeLabel': 'Volumen general',
  'settings.audio.previewVolume': 'Volumen de previsualizacion — {percent}%',
  'settings.audio.previewVolumeLabel': 'Volumen de previsualizacion',
  'settings.audio.previewVolumeHint':
    'Se usa al escuchar audios desde la biblioteca, sin afectar la reproduccion de los botones.',
  'settings.audio.playbackMode': 'Modo de reproduccion',
  'settings.audio.playbackModeHint':
    'Interrumpir corta el sonido anterior; superponer permite varios a la vez.',
  'settings.audio.interrupt': 'Interrumpir',
  'settings.audio.overlap': 'Superponer',
  'settings.audio.restartSame': 'Reiniciar el mismo audio',
  'settings.audio.restartSameHint':
    'Volver a disparar un sonido que ya suena lo empieza desde el principio.',
  'settings.audio.normalize': 'Igualar el volumen entre audios',
  'settings.audio.normalizeHint':
    'Compensa la diferencia de volumen con el que fue grabado cada audio, sin tocar los volumenes propios que hayas puesto.',
  'settings.audio.measure': 'Medir la biblioteca',
  'settings.audio.measureHint':
    'Los audios que entraron antes de activar esto no estan medidos y suenan como siempre hasta que los midas.',
  'settings.audio.measureAction': 'Medir',
  'settings.audio.measuring': 'Midiendo...',
  'settings.audio.measuredNone': 'Ya estaban todos medidos.',
  'settings.audio.measured': '{count} audios medidos{failed}.',
  'settings.audio.measureFailed': ', {count} no se pudieron medir',
  'settings.audio.showGuide': 'Mostrar guia de dispositivo virtual',
  'settings.audio.hideGuide': 'Ocultar guia de dispositivo virtual',
  'settings.audio.guideIntro':
    'Sound Deck no crea un microfono virtual. Solo reproduce en el dispositivo de salida que elijas.',
  'settings.audio.guideSpeakers': 'Parlantes normales',
  'settings.audio.guideDiscord': 'Discord y juegos',
  'settings.audio.guideDiscordHint':
    'Instala un dispositivo virtual de salida (VB-Cable o equivalente), elegilo aca como salida y configura su entrada correspondiente como microfono en Discord.',
  'settings.audio.guideObs': 'OBS',

  // --- Ajustes: atajos -------------------------------------------------------
  'settings.shortcuts.intro':
    'Los atajos globales funcionan en todo el sistema. Los de overlay solo mientras el overlay tiene el foco. Las teclas 1 a 9 reproducen los botones dentro del overlay.',
  'settings.shortcuts.scopeGlobal': 'Global — todo el sistema',
  'settings.shortcuts.scopeOverlay': 'Solo dentro del overlay',
  'settings.shortcuts.change': 'Cambiar atajo de {action}',
  'settings.shortcuts.pressCombo': 'Presiona la combinacion...',
  'settings.shortcuts.updated': 'Atajo actualizado: {accelerator}',
  'settings.shortcuts.reset': 'Restaurar atajos predeterminados',
  'settings.shortcuts.resetDone': 'Atajos restablecidos a los valores predeterminados.',
  'settings.shortcuts.globalSlots': 'Reproducir los botones 1 a 9 en todo el sistema',
  'settings.shortcuts.globalSlotsHint':
    'Sin abrir el overlay. Las nueve combinaciones quedan tomadas mientras este activado.',
  'settings.shortcuts.slotModifier': 'Combinacion de los botones',
  'settings.shortcuts.slotModifierLabel': 'Combinacion de los botones 1 a 9',
  'settings.shortcuts.slotModifierHint': 'Queda {first} hasta {last}.',
  'shortcut.toggle_overlay': 'Abrir/cerrar overlay',
  'shortcut.stop_all': 'Detener todos los sonidos',
  'shortcut.prev_page': 'Pagina anterior',
  'shortcut.next_page': 'Pagina siguiente',

  // --- Ajustes: biblioteca ---------------------------------------------------
  'settings.library.soundsFolder': 'Carpeta de sonidos',
  'settings.library.usedSpace': 'Espacio utilizado',
  'settings.library.usedSpaceHint': '{size} en la carpeta administrada.',
  'settings.library.calculating': 'Calculando...',
  'settings.library.cleanTemp': 'Limpiar temporales',
  'settings.library.cleanTempHint': 'Borra restos de descargas o importaciones interrumpidas.',
  'settings.library.clean': 'Limpiar',
  'settings.library.cleanedTemp': '{count} archivos temporales borrados.',
  'settings.library.noTemp': 'No habia temporales.',
  'settings.library.findMissing': 'Buscar archivos faltantes',
  'settings.library.findMissingHint': 'Revisa si algun audio de la biblioteca perdio su archivo.',
  'settings.library.missingCount': '{count} audios apuntan a archivos que ya no existen.',
  'settings.library.check': 'Revisar',
  'settings.library.missingFound': '{count} audios sin archivo.',
  'settings.library.noneMissing': 'Todos los audios tienen su archivo.',
  'settings.library.removeOrphans': 'Eliminar registros huerfanos',
  'settings.library.removeOrphansHint':
    'Quita de la biblioteca los audios cuyo archivo ya no existe. Tambien libera sus botones.',
  'settings.library.orphansRemoved': '{count} registros eliminados.',
  'settings.library.noOrphans': 'No habia registros huerfanos.',
  'settings.library.backup': 'Copia de seguridad',
  'settings.library.backupHint': 'Guarda una copia de la base de datos en la carpeta backups.',
  'settings.library.backupDone': 'Copia de seguridad creada.',
  'settings.library.restore': 'Restaurar copia',
  'settings.library.restoreHint':
    'Reemplaza paginas, botones y audios registrados por los de la copia. Los archivos de audio en disco no se tocan.',
  'settings.library.restoreTitle': 'Restaurar copia de seguridad',
  'settings.library.restoreFilter': 'Base de datos',
  'settings.library.restoreConfirm':
    'Se va a reemplazar tu biblioteca actual por la de la copia. La actual queda guardada en la carpeta backups. Sound Deck se reinicia para terminar.',
  'settings.library.restoreOk': 'Restaurar',

  // --- Ajustes: proveedores --------------------------------------------------
  'settings.providers.unofficial': 'No oficial',
  'settings.providers.ready': 'Listo',
  'settings.providers.terms': 'Ver terminos y condiciones del servicio',
  'settings.providers.enable': 'Activar {provider}',
  'settings.providers.apiKey': 'API key',
  'settings.providers.apiKeySaved': 'Guardada ({masked}). Escribi una nueva para reemplazarla.',
  'settings.providers.apiKeyHint':
    'Necesaria para buscar. Se guarda solo en tu computadora y nunca aparece en los logs.',
  'settings.providers.apiKeyPlaceholder': 'Pega tu API key',
  'settings.providers.apiKeyLabel': 'API key de {provider}',
  'settings.providers.apiKeySavedToast': 'API key guardada.',
  'settings.providers.testConnection': 'Probar conexion',
  'settings.providers.connectionOk': 'Conexion correcta.',
  'settings.providers.accountConnected': 'Cuenta conectada.',
  'settings.providers.accountConnectedHint':
    'Las descargas traen el archivo original, en su formato y calidad de subida.',
  'settings.providers.disconnect': 'Desconectar',
  'settings.providers.disconnected': 'Cuenta desconectada. Se vuelve a guardar la preview.',
  'settings.providers.connected':
    'Cuenta conectada. Las descargas van a traer el archivo original.',
  'settings.providers.accountIntro':
    'Sin conectar tu cuenta se guarda la preview MP3, que alcanza para una soundboard. Conectandola se baja el archivo original. Hace falta que en tu credencial de Freesound elijas la opcion que muestra el codigo en pantalla.',
  'settings.providers.clientId': 'Client id',
  'settings.providers.clientIdSaved': 'Guardado. Escribi uno nuevo para reemplazarlo.',
  'settings.providers.clientIdHint': 'Esta en la misma pagina que la API key, arriba de ella.',
  'settings.providers.clientIdPlaceholder': 'Pega tu Client id',
  'settings.providers.clientIdLabel': 'Client id de {provider}',
  'settings.providers.authorize': 'Autorizar en Freesound',
  'settings.providers.code': 'Codigo de autorizacion',
  'settings.providers.codeHint': 'Duran 10 minutos y sirven una sola vez.',
  'settings.providers.codePlaceholder': 'Pega el codigo que te muestra Freesound',
  'settings.providers.codeLabel': 'Codigo de autorizacion de Freesound',
  'settings.providers.connect': 'Conectar',

  // --- Ajustes: avanzado -----------------------------------------------------
  'settings.advanced.logsFolder': 'Carpeta de logs',
  'settings.advanced.logLevel': 'Nivel de logs',
  'settings.advanced.logLevelHint': 'Cambia el detalle registrado. Se aplica al instante.',
  'settings.advanced.logError': 'Error',
  'settings.advanced.logWarn': 'Advertencia',
  'settings.advanced.logInfo': 'Info',
  'settings.advanced.logDebug': 'Debug',
  'settings.advanced.logTrace': 'Trace',
  'settings.advanced.reset': 'Restablecer configuracion',
  'settings.advanced.resetHint':
    'Vuelve a los valores predeterminados. No borra audios ni paginas.',
  'settings.advanced.resetAction': 'Restablecer',
  'settings.advanced.resetDone': 'Configuracion restablecida.',

  // --- Ajustes: creditos -----------------------------------------------------
  'settings.credits.tagline':
    'Soundboard de escritorio local-first. Tus audios viven en tu computadora y funcionan sin conexion. Codigo bajo licencia MIT.',
  'settings.credits.contentTitle': 'Sobre el contenido',
  'settings.credits.content':
    'Sound Deck no incluye ningun audio de terceros: el tono de prueba se genera en memoria. Los audios que descargues conservan la licencia de su origen, que queda guardada en la metadata de cada sonido junto con su atribucion. Revisar que podes hacer con cada uno antes de usarlo en algo publico queda de tu lado.',
  'settings.credits.groupApp': 'Aplicacion',
  'settings.credits.groupAudio': 'Audio',
  'settings.credits.groupData': 'Datos y red',
  'settings.credits.authorBefore': 'Desarrollado por ',
  'settings.credits.authorAfter': ' con ayuda de Claude.',
  'settings.credits.openDataFolder': 'Abrir carpeta de datos',

  // --- Ajustes: diagramas de la guia de audio --------------------------------
  'settings.audio.diagramSpeakers': 'Sound Deck  ->  Parlantes / auriculares',
  'settings.audio.diagramDiscord': `Sound Deck  ->  Salida virtual (ej. CABLE Input)
                        |
                        v
Discord: microfono = entrada virtual (CABLE Output)`,
  'settings.audio.diagramObs': `Sound Deck  ->  Salida virtual
OBS: agregar "Captura de entrada de audio" -> entrada virtual`,

  // --- Ajustes: ayuda de cada proveedor --------------------------------------
  'help.freesound.title': 'Como sacar la API key',
  'help.freesound.step1':
    'Crea una cuenta gratuita en freesound.org y entra a freesound.org/apiv2/apply.',
  'help.freesound.step2': 'Completa solo el nombre y la descripcion de la aplicacion.',
  'help.freesound.step3':
    'El formulario pide una URL de callback. Si solo vas a buscar y bajar previews, dejala vacia. Si ademas queres conectar tu cuenta para bajar los archivos originales, elegi la opcion de Freesound que muestra el codigo en pantalla.',
  'help.freesound.step4':
    'Al guardar, las credenciales aparecen al instante. Para buscar alcanza con el API key; para conectar la cuenta hace falta tambien el Client id, que esta justo arriba.',
  'help.freesound.note':
    'Freesound es un banco de sonidos y efectos con licencias claras. No es un sitio de memes: si buscas audios de ese tipo, vas a encontrar mas en un proveedor no oficial.',
  'help.myinstants.title': 'No necesita configuracion',
  'help.myinstants.note':
    'Alcanza con activarlo. Sound Deck consulta las mismas paginas publicas de busqueda que verias en un navegador, solo cuando escribis algo, y espaciando las consultas.',
  'help.myinstants.warning':
    'MyInstants no tiene API oficial: si el sitio cambia su estructura, este proveedor puede dejar de funcionar de un dia para el otro. Los audios los sube la comunidad y no declaran licencia, asi que revisa vos que podes hacer con cada uno antes de usarlo en algo publico.',

  // --- Ajustes: para que sirve cada dependencia ------------------------------
  'credits.tauri': 'Ventanas nativas, bandeja y atajos globales',
  'credits.react': 'Interfaz',
  'credits.vite': 'Build y desarrollo',
  'credits.tailwind': 'Estilos',
  'credits.radix': 'Primitivas accesibles',
  'credits.lucide': 'Iconos',
  'credits.tanstack': 'Estado asincronico y listas virtualizadas',
  'credits.zustand': 'Estado de interfaz',
  'credits.rodio': 'Decodificacion y mezcla',
  'credits.cpal': 'Enumeracion y apertura de dispositivos',
  'credits.symphonia': 'Decodificadores MP3, FLAC, Vorbis y WAV',
  'credits.ebur128': 'Medicion de sonoridad para normalizar el volumen',
  'credits.sqlite': 'Base de datos local embebida',
  'credits.reqwest': 'Descargas y consultas a proveedores',
  'credits.scraper': 'Parseo HTML del proveedor no oficial',
  'credits.sha2': 'Hash de contenido para deduplicar',

  'toast.imageRemoved': 'Imagen quitada.',
  'toast.soundDeleted': 'Audio eliminado de la biblioteca.',
  'toast.soundSaved': '"{name}" guardado en la biblioteca.',
  'toast.importedNone': 'No se importo ningun archivo.',
  'toast.importedSounds.one': '{count} audio importado',
  'toast.importedSounds.many': '{count} audios importados',
  'toast.importedDuplicates': '{count} ya estaban en la biblioteca',
  'toast.importedFailed': '{count} con error',
} as const;
