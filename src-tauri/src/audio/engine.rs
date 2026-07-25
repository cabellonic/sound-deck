//! Motor de reproduccion (§18).
//!
//! Mantiene un unico stream de salida abierto sobre el dispositivo elegido y un
//! conjunto de reproducciones activas. La previsualizacion vive aparte de la
//! reproduccion principal: son sesiones independientes con su propio volumen.
//!
//! Todas las operaciones son bloqueantes y cortas (abrir archivo, encolar en el
//! mixer). Los comandos Tauri las invocan desde un hilo de blocking, nunca desde
//! el hilo principal.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use rodio::source::EmptyCallback;
use rodio::{Decoder, MixerDeviceSink, Player};
use tauri::AppHandle;

use crate::domain::settings::PlaybackMode;
use crate::errors::{AppError, AppResult, ErrorKind};
use crate::events::{self, DeviceChangedPayload, PlaybackPayload};

use super::devices::{resolve_device, AudioDeviceInfo, DeviceResolution};

/// Una reproduccion en curso.
struct ActivePlayback {
    /// `None` en previsualizaciones y en la prueba de dispositivo.
    sound_id: Option<String>,
    player: Player,
}

impl ActivePlayback {
    fn finished(&self) -> bool {
        self.player.empty()
    }
}

/// Estado interno protegido por un unico mutex.
struct EngineInner {
    /// Stream abierto. `None` mientras no se pudo abrir ningun dispositivo.
    sink: Option<MixerDeviceSink>,
    device: Option<AudioDeviceInfo>,
    playing: Vec<ActivePlayback>,
    preview: Option<ActivePlayback>,
    /// Archivo temporal de la preview remota en curso, para borrarlo al parar.
    preview_temp_file: Option<PathBuf>,
}

impl EngineInner {
    /// Descarta las reproducciones que ya terminaron.
    fn prune(&mut self) {
        self.playing.retain(|playback| !playback.finished());
        if self
            .preview
            .as_ref()
            .map(ActivePlayback::finished)
            .unwrap_or(false)
        {
            self.preview = None;
            Self::remove_preview_temp(&mut self.preview_temp_file);
        }
    }

    fn remove_preview_temp(slot: &mut Option<PathBuf>) {
        if let Some(path) = slot.take() {
            if let Err(error) = std::fs::remove_file(&path) {
                tracing::debug!(path = %path.display(), %error, "no se pudo borrar la preview temporal");
            }
        }
    }
}

/// Descarta las previsualizaciones que llegan tarde.
///
/// Una previsualizacion online tiene que bajarse antes de sonar, y en ese rato
/// el usuario puede pedir otra. Cada pedido se lleva un numero; cuando su
/// descarga termina, solo suena si el numero sigue siendo el ultimo. Sin esto,
/// una descarga lenta puede terminar despues de una rapida y sonar encima.
#[derive(Debug, Default)]
pub struct PreviewGate(AtomicU64);

impl PreviewGate {
    /// Invalida lo que haya en vuelo y devuelve el numero del pedido nuevo.
    pub fn invalidate(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }

    pub fn is_current(&self, token: u64) -> bool {
        self.current() == token
    }
}

/// Motor de audio compartido. Clonar comparte el mismo estado.
#[derive(Clone)]
pub struct AudioEngine {
    inner: Arc<Mutex<EngineInner>>,
    app: AppHandle,
    preview_gate: Arc<PreviewGate>,
}

impl AudioEngine {
    pub fn new(app: AppHandle) -> Self {
        Self {
            inner: Arc::new(Mutex::new(EngineInner {
                sink: None,
                device: None,
                playing: Vec::new(),
                preview: None,
                preview_temp_file: None,
            })),
            app,
            preview_gate: Arc::new(PreviewGate::default()),
        }
    }

    /// Anuncia una previsualizacion que todavia tiene que cargarse.
    ///
    /// Corta lo que estuviera sonando, que es lo que el usuario espera al
    /// apretar play en otra cosa, y devuelve el numero de este pedido.
    pub fn begin_preview(&self) -> u64 {
        self.stop_preview();
        self.preview_gate.current()
    }

    /// Si el pedido sigue siendo el ultimo, o alguien pidio otra cosa mientras.
    pub fn is_current_preview(&self, token: u64) -> bool {
        self.preview_gate.is_current(token)
    }

    /// Dispositivo actualmente en uso.
    pub fn current_device(&self) -> Option<AudioDeviceInfo> {
        self.inner.lock().device.clone()
    }

    /// Abre el dispositivo indicado por la preferencia guardada.
    ///
    /// Devuelve la informacion del dispositivo abierto y como se resolvio, para
    /// que quien llama decida si avisar al usuario.
    pub fn open_device(
        &self,
        preferred_id: Option<&str>,
        preferred_name: Option<&str>,
    ) -> AppResult<(AudioDeviceInfo, DeviceResolution)> {
        let resolved = resolve_device(preferred_id, preferred_name)?;
        let info = resolved.info.clone();

        let sink = rodio::stream::DeviceSinkBuilder::from_device(resolved.device)
            .and_then(|builder| builder.open_stream())
            .map_err(|error| {
                AppError::new(
                    ErrorKind::AudioDevice,
                    format!(
                        "No se pudo abrir el dispositivo \u{201c}{}\u{201d}. Puede estar en uso o haberse desconectado.",
                        info.name
                    ),
                )
                .with_technical(error.to_string())
                .with_detail("deviceName", info.name.clone())
            })?;

        {
            let mut inner = self.inner.lock();
            // Cerrar el stream anterior detiene lo que estuviera sonando.
            inner.playing.clear();
            inner.preview = None;
            EngineInner::remove_preview_temp(&mut inner.preview_temp_file);
            inner.sink = Some(sink);
            inner.device = Some(resolved.info.clone());
        }

        tracing::info!(
            device = %resolved.info.name,
            id = ?resolved.info.id,
            resolution = ?resolved.resolution,
            "dispositivo de salida abierto"
        );

        events::emit(
            &self.app,
            events::AUDIO_DEVICE_CHANGED,
            DeviceChangedPayload {
                device_name: resolved.info.name.clone(),
                device_id: resolved.info.id.clone(),
                notice: resolved.resolution.notice(preferred_name),
            },
        );

        Ok((resolved.info, resolved.resolution))
    }

    /// Reproduce un archivo local ya validado.
    ///
    /// - `mode` decide si corta lo anterior o se superpone.
    /// - `restart` reinicia el mismo sonido si ya estaba sonando.
    pub fn play_file(
        &self,
        path: &Path,
        volume: f32,
        mode: PlaybackMode,
        sound_id: Option<String>,
        restart: bool,
    ) -> AppResult<()> {
        let source = decode_file(path)?;

        let mut inner = self.inner.lock();
        inner.prune();

        if inner.sink.is_none() {
            return Err(AppError::new(
                ErrorKind::AudioDevice,
                "No hay un dispositivo de salida abierto. Elegi uno en Ajustes > Audio.",
            ));
        }

        match mode {
            PlaybackMode::Interrupt => inner_stop_all(&mut inner),
            PlaybackMode::Overlap => {
                if restart {
                    if let Some(sound_id) = &sound_id {
                        // Reiniciar el mismo sonido: cortamos su instancia previa.
                        inner.playing.retain(|playback| {
                            let same = playback.sound_id.as_ref() == Some(sound_id);
                            if same {
                                playback.player.stop();
                            }
                            !same
                        });
                    }
                }
            }
        }

        let sink = inner
            .sink
            .as_ref()
            .expect("verificado arriba dentro del mismo lock");

        let player = Player::connect_new(sink.mixer());
        player.set_volume(volume);
        player.append(source);
        // Encolado detras del audio: se consume cuando este termina de sonar.
        player.append(completion_notifier(&self.app, sound_id.clone(), false));

        inner.playing.push(ActivePlayback {
            sound_id: sound_id.clone(),
            player,
        });
        drop(inner);

        events::emit(
            &self.app,
            events::PLAYBACK_STARTED,
            PlaybackPayload {
                sound_id,
                is_preview: false,
            },
        );
        Ok(())
    }

    /// Previsualiza un archivo. Solo puede haber una preview a la vez (§27).
    ///
    /// `temp_file` indica un archivo temporal que debe borrarse cuando la
    /// preview termine (el caso de un audio remoto todavia no guardado).
    pub fn preview_file(
        &self,
        path: &Path,
        volume: f32,
        temp_file: Option<PathBuf>,
    ) -> AppResult<()> {
        self.preview_gate.invalidate();
        let source = decode_file(path)?;

        let mut inner = self.inner.lock();
        inner.prune();

        if inner.sink.is_none() {
            return Err(AppError::new(
                ErrorKind::AudioDevice,
                "No hay un dispositivo de salida abierto. Elegi uno en Ajustes > Audio.",
            ));
        }

        if let Some(previous) = inner.preview.take() {
            previous.player.stop();
        }
        EngineInner::remove_preview_temp(&mut inner.preview_temp_file);

        let sink = inner
            .sink
            .as_ref()
            .expect("verificado arriba dentro del mismo lock");
        let player = Player::connect_new(sink.mixer());
        player.set_volume(volume);
        player.append(source);
        player.append(completion_notifier(&self.app, None, true));

        inner.preview = Some(ActivePlayback {
            sound_id: None,
            player,
        });
        inner.preview_temp_file = temp_file;
        drop(inner);

        events::emit(
            &self.app,
            events::PLAYBACK_STARTED,
            PlaybackPayload {
                sound_id: None,
                is_preview: true,
            },
        );
        Ok(())
    }

    pub fn stop_preview(&self) {
        self.preview_gate.invalidate();
        let mut inner = self.inner.lock();
        let had_preview = inner.preview.take().inspect(|p| p.player.stop()).is_some();
        EngineInner::remove_preview_temp(&mut inner.preview_temp_file);
        drop(inner);

        if had_preview {
            events::emit(
                &self.app,
                events::PLAYBACK_STOPPED,
                PlaybackPayload {
                    sound_id: None,
                    is_preview: true,
                },
            );
        }
    }

    /// Detiene todo: reproducciones y previsualizacion.
    pub fn stop_all(&self) {
        self.preview_gate.invalidate();
        let mut inner = self.inner.lock();
        inner_stop_all(&mut inner);
        if let Some(preview) = inner.preview.take() {
            preview.player.stop();
        }
        EngineInner::remove_preview_temp(&mut inner.preview_temp_file);
        drop(inner);

        events::emit(
            &self.app,
            events::PLAYBACK_STOPPED,
            PlaybackPayload {
                sound_id: None,
                is_preview: false,
            },
        );
    }

    /// Ids de los sonidos que estan sonando ahora mismo.
    pub fn playing_sound_ids(&self) -> Vec<String> {
        let mut inner = self.inner.lock();
        inner.prune();
        inner
            .playing
            .iter()
            .filter_map(|playback| playback.sound_id.clone())
            .collect()
    }

    pub fn is_previewing(&self) -> bool {
        let mut inner = self.inner.lock();
        inner.prune();
        inner.preview.is_some()
    }

    /// Reproduce un tono corto generado en memoria para probar el dispositivo.
    /// No incluimos ningun audio de terceros en el repositorio (§32).
    pub fn play_test_tone(&self, volume: f32) -> AppResult<()> {
        use rodio::source::{SineWave, Source};
        use std::time::Duration;

        let mut inner = self.inner.lock();
        inner.prune();

        let Some(sink) = inner.sink.as_ref() else {
            return Err(AppError::new(
                ErrorKind::AudioDevice,
                "No hay un dispositivo de salida abierto. Elegi uno en Ajustes > Audio.",
            ));
        };

        // Tono de 440 Hz con fundido de entrada y salida para que no chasquee.
        let tone = SineWave::new(440.0)
            .take_duration(Duration::from_millis(450))
            .fade_in(Duration::from_millis(40))
            .amplify(0.6);

        let player = Player::connect_new(sink.mixer());
        player.set_volume(volume);
        player.append(tone);

        inner.playing.push(ActivePlayback {
            sound_id: None,
            player,
        });
        Ok(())
    }
}

fn inner_stop_all(inner: &mut EngineInner) {
    for playback in inner.playing.drain(..) {
        playback.player.stop();
    }
}

/// Fuente vacia que avisa cuando el audio anterior termino solo.
///
/// Es la alternativa a hacer polling desde el frontend (§24): rodio la consume
/// justo despues del ultimo sample, asi que el evento llega en el momento real
/// en que el sonido se apaga. Si en cambio se corta con `stop()`, la cola se
/// vacia y este callback no corre: ese caso ya emite su propio evento.
fn completion_notifier(
    app: &AppHandle,
    sound_id: Option<String>,
    is_preview: bool,
) -> EmptyCallback {
    let app = app.clone();
    // La cola podria sondear la fuente mas de una vez; avisamos una sola.
    let notified = AtomicBool::new(false);

    EmptyCallback::new(Box::new(move || {
        if notified.swap(true, Ordering::SeqCst) {
            return;
        }

        events::emit(
            &app,
            events::PLAYBACK_STOPPED,
            PlaybackPayload {
                sound_id: sound_id.clone(),
                is_preview,
            },
        );
    }))
}

/// Abre y decodifica un archivo. Un archivo borrado o corrupto produce un error
/// accionable con el detalle necesario para ofrecer "quitar asignacion" (§29).
fn decode_file(path: &Path) -> AppResult<Decoder<std::io::BufReader<std::fs::File>>> {
    let file = std::fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::new(
                ErrorKind::Playback,
                "El archivo de audio ya no esta en la carpeta de la aplicacion.",
            )
            .with_detail("reason", "missing_file")
            .with_technical(format!("{}: {error}", path.display()))
        } else {
            AppError::new(ErrorKind::Playback, "No se pudo abrir el archivo de audio.")
                .with_technical(format!("{}: {error}", path.display()))
        }
    })?;

    Decoder::try_from(file).map_err(|error| {
        AppError::new(
            ErrorKind::InvalidAudio,
            "El archivo de audio esta danado y no se puede reproducir.",
        )
        .with_detail("reason", "corrupt_file")
        .with_technical(format!("{}: {error}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Decoder` no implementa `Debug`, asi que extraemos el error a mano.
    fn error_de(path: &Path) -> AppError {
        match decode_file(path) {
            Ok(_) => panic!("se esperaba un error al decodificar {}", path.display()),
            Err(error) => error,
        }
    }

    #[test]
    fn solo_suena_la_ultima_previsualizacion_pedida() {
        let gate = PreviewGate::default();

        // Se pide una y todavia no llego nadie mas: cuando termine, suena.
        let primera = gate.invalidate();
        assert!(gate.is_current(primera));

        // El usuario aprieta play en otra mientras la primera se baja.
        let segunda = gate.invalidate();
        assert!(gate.is_current(segunda));
        assert!(
            !gate.is_current(primera),
            "la primera llego tarde y no deberia sonar"
        );
    }

    #[test]
    fn detener_invalida_lo_que_se_estaba_bajando() {
        let gate = PreviewGate::default();
        let pedido = gate.invalidate();

        // Equivale a apretar stop, reproducir algo local o detener todo.
        gate.invalidate();

        assert!(!gate.is_current(pedido));
    }

    #[test]
    fn los_numeros_de_pedido_no_se_repiten() {
        // Si se reciclaran, una descarga vieja podria hacerse pasar por la
        // actual justo despues de dar la vuelta.
        let gate = PreviewGate::default();
        let emitidos: Vec<u64> = (0..100).map(|_| gate.invalidate()).collect();

        let mut unicos = emitidos.clone();
        unicos.sort_unstable();
        unicos.dedup();
        assert_eq!(unicos.len(), emitidos.len());
        assert!(emitidos.windows(2).all(|par| par[1] > par[0]));
    }

    #[test]
    fn decodificar_un_archivo_inexistente_da_un_error_accionable() {
        let error = error_de(Path::new("no-existe-jamas.mp3"));
        assert_eq!(error.kind, ErrorKind::Playback);
        assert_eq!(
            error.details.get("reason").map(String::as_str),
            Some("missing_file")
        );
        // El mensaje al usuario no filtra rutas tecnicas crudas.
        assert!(!error.message.contains("no-existe-jamas"));
    }

    #[test]
    fn decodificar_basura_reporta_audio_invalido() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("basura.mp3");
        std::fs::write(&ruta, b"esto no es audio de ninguna manera").unwrap();

        let error = error_de(&ruta);
        assert_eq!(error.kind, ErrorKind::InvalidAudio);
        assert_eq!(
            error.details.get("reason").map(String::as_str),
            Some("corrupt_file")
        );
    }
}
